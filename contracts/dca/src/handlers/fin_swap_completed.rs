use crate::constants::ONE_HUNDRED;
use crate::error::ContractError;
use crate::state::{create_event, trigger_store, vault_store, CACHE, CONFIG, TIME_TRIGGER_CACHE};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData, ExecutionSkippedReason};
use base::helpers::message_helpers::{find_first_attribute_by_key, find_first_event_by_type};
use base::helpers::time_helpers::get_next_target_time;
use base::triggers::trigger::TriggerConfiguration;
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
    let time_trigger_cache = TIME_TRIGGER_CACHE.load(deps.storage)?;
    let vault = vault_store().load(deps.storage, cache.vault_id.into())?;
    let trigger_store = trigger_store();
    let trigger = trigger_store.load(deps.storage, time_trigger_cache.trigger_id.into())?;

    let mut attributes: Vec<Attribute> = Vec::new();
    let mut messages: Vec<CosmosMsg> = Vec::new();

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

            let funds_to_redistribute = Coin::new(
                (coin_received.amount - execution_fee.amount).into(),
                &coin_received.denom,
            );

            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![funds_to_redistribute],
            }));

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

                            if let true = existing_vault.low_funds() {
                                existing_vault.status = VaultStatus::Inactive;
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
                TriggerConfiguration::Time { mut target_time } => {
                    let next_trigger_time =
                        get_next_target_time(env.block.time, target_time, vault.time_interval);

                    trigger_store.update(deps.storage, trigger.id.into(), |existing_trigger| {
                        match existing_trigger {
                            Some(trigger) => {
                                target_time = next_trigger_time;
                                Ok(trigger)
                            }
                            None => Err(ContractError::CustomError {
                                val: format!("could not trigger with id: {}", trigger.id),
                            }),
                        }
                    })?;
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
                TriggerConfiguration::Time { mut target_time } => {
                    let next_trigger_time =
                        get_next_target_time(env.block.time, target_time, vault.time_interval);

                    trigger_store.update(deps.storage, trigger.id.into(), |existing_trigger| {
                        match existing_trigger {
                            Some(trigger) => {
                                target_time = next_trigger_time;
                                Ok(trigger)
                            }
                            None => Err(ContractError::CustomError {
                                val: format!("could not find trigger with id: {}", trigger.id),
                            }),
                        }
                    })?;
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
