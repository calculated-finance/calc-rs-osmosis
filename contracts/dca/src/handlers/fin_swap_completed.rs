use crate::constants::ONE_HUNDRED;
use crate::error::ContractError;
use crate::state::{
    create_event, get_trigger, remove_trigger, save_trigger, vault_store, CACHE, CONFIG,
};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData, ExecutionSkippedReason};
use base::helpers::message_helpers::{find_first_attribute_by_key, find_first_event_by_type};
use base::helpers::time_helpers::get_next_target_time;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use base::vaults::vault::{PositionType, VaultStatus};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Attribute, BankMsg, Coin, CosmosMsg, DepsMut, Env, Reply, Response, Uint128};
use fin_helpers::codes::{ERROR_SWAP_INSUFFICIENT_FUNDS, ERROR_SWAP_SLIPPAGE};

pub fn fin_swap_completed(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = vault_store().load(deps.storage, cache.vault_id.into())?;
    let trigger = get_trigger(deps.storage, vault.id.into())?;

    let mut attributes: Vec<Attribute> = Vec::new();
    let mut messages: Vec<CosmosMsg> = Vec::new();

    remove_trigger(deps.storage, vault.id)?;

    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_swap_response = reply.result.into_result().unwrap();

            let wasm_trade_event =
                find_first_event_by_type(&fin_swap_response.events, "wasm-trade").unwrap();

            let base_amount =
                find_first_attribute_by_key(&wasm_trade_event.attributes, "base_amount")
                    .unwrap()
                    .value
                    .parse::<u128>()
                    .unwrap();

            let quote_amount =
                find_first_attribute_by_key(&wasm_trade_event.attributes, "quote_amount")
                    .unwrap()
                    .value
                    .parse::<u128>()
                    .unwrap();

            let (coin_sent, coin_received) = match vault.position_type {
                PositionType::Enter => {
                    let sent = Coin {
                        denom: vault.get_swap_denom(),
                        amount: Uint128::from(quote_amount),
                    };
                    let received = Coin {
                        denom: vault.get_receive_denom(),
                        amount: Uint128::from(base_amount),
                    };

                    (sent, received)
                }
                PositionType::Exit => {
                    let sent = Coin {
                        denom: vault.get_swap_denom(),
                        amount: Uint128::from(base_amount),
                    };
                    let received = Coin {
                        denom: vault.get_receive_denom(),
                        amount: Uint128::from(quote_amount),
                    };

                    (sent, received)
                }
            };

            let config = CONFIG.load(deps.storage)?;

            let execution_fee = Coin::new(
                (coin_received
                    .amount
                    .checked_multiply_ratio(config.fee_percent, ONE_HUNDRED)?)
                .into(),
                &coin_received.denom,
            );

            let total_to_redistribute = coin_received.amount - execution_fee.amount;

            vault
                .destinations
                .iter()
                .map(|destination| BankMsg::Send {
                    to_address: destination.address.to_string(),
                    amount: vec![Coin::new(
                        total_to_redistribute
                            .checked_multiply_ratio(
                                destination.allocation.atomics(),
                                Uint128::new(10)
                                    .checked_pow(destination.allocation.decimal_places())
                                    .unwrap(),
                            )
                            .unwrap()
                            .u128(),
                        &coin_received.denom,
                    )],
                })
                .into_iter()
                .for_each(|msg| messages.push(CosmosMsg::Bank(msg.to_owned())));

            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: config.fee_collector.to_string(),
                amount: vec![execution_fee.clone()],
            }));

            vault_store().update(
                deps.storage,
                vault.id.into(),
                |existing_vault| -> Result<Vault, ContractError> {
                    match existing_vault {
                        Some(mut existing_vault) => {
                            existing_vault.balance.amount -=
                                existing_vault.get_swap_amount().amount;

                            if existing_vault.low_funds() {
                                existing_vault.status = VaultStatus::Inactive;
                            }

                            if existing_vault.started_at.is_none() {
                                existing_vault.started_at = Some(env.block.time);
                            }

                            Ok(existing_vault)
                        }
                        None => Err(ContractError::CustomError {
                            val: format!(
                                "could not find vault for address: {} with id: {}",
                                vault.owner.clone(),
                                vault.id
                            ),
                        }),
                    }
                },
            )?;

            match trigger.configuration {
                TriggerConfiguration::Time { target_time } => {
                    save_trigger(
                        deps.storage,
                        Trigger {
                            vault_id: vault.id,
                            configuration: TriggerConfiguration::Time {
                                target_time: get_next_target_time(
                                    env.block.time,
                                    target_time,
                                    vault.time_interval,
                                ),
                            },
                        },
                    )?;
                }
                _ => panic!("should be a time based trigger"),
            }

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block,
                    EventData::DCAVaultExecutionCompleted {
                        sent: coin_sent.clone(),
                        received: coin_received.clone(),
                        fee: execution_fee,
                    },
                ),
            )?;

            attributes.push(Attribute::new("status", "success"));
        }
        cosmwasm_std::SubMsgResult::Err(e) => {
            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block.to_owned(),
                    EventData::DCAVaultExecutionSkipped {
                        reason: if e.contains(ERROR_SWAP_SLIPPAGE) {
                            ExecutionSkippedReason::SlippageToleranceExceeded
                        } else if e.contains(ERROR_SWAP_INSUFFICIENT_FUNDS) {
                            ExecutionSkippedReason::InsufficientFunds
                        } else {
                            ExecutionSkippedReason::UnknownFailure
                        },
                    },
                ),
            )?;

            attributes.push(Attribute::new("status", "skipped"));

            match trigger.configuration {
                TriggerConfiguration::Time { target_time } => {
                    save_trigger(
                        deps.storage,
                        Trigger {
                            vault_id: vault.id,
                            configuration: TriggerConfiguration::Time {
                                target_time: get_next_target_time(
                                    env.block.time,
                                    target_time,
                                    vault.time_interval,
                                ),
                            },
                        },
                    )?;
                }
                _ => panic!("should be a time based trigger"),
            }
        }
    };

    CACHE.remove(deps.storage);

    Ok(Response::new()
        .add_attribute("method", "after_fin_swap_completed")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_attributes(attributes)
        .add_messages(messages))
}
