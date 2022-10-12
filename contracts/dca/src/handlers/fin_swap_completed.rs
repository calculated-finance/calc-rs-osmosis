use crate::error::ContractError;
use crate::state::{save_event, trigger_store, vault_store, CACHE};
use base::events::event::{EventBuilder, EventData, ExecutionSkippedReason};
use base::helpers::message_helpers::{find_first_attribute_by_key, find_first_event_by_type};
use base::helpers::time_helpers::get_next_target_time;
use base::triggers::trigger::TriggerConfiguration;
use base::vaults::vault::{PositionType, Vault, VaultConfiguration};
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
    let trigger_store = trigger_store();
    let trigger = trigger_store.load(deps.storage, vault.trigger_id.unwrap().into())?;

    let mut attributes: Vec<Attribute> = Vec::new();
    let mut messages: Vec<CosmosMsg> = Vec::new();

    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => match &vault.configuration {
            VaultConfiguration::DCA {
                pair: _,
                swap_amount: _,
                position_type,
                slippage_tolerance: _,
            } => {
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

                let (coin_sent, coin_received) = match position_type {
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

                let bank_msg_to_vault_owner: BankMsg = BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![coin_received.clone()],
                };

                messages.push(CosmosMsg::Bank(bank_msg_to_vault_owner));

                vault_store().update(
                    deps.storage,
                    vault.id.into(),
                    |existing_vault| -> Result<Vault, ContractError> {
                        match existing_vault {
                            Some(mut existing_vault) => {
                                existing_vault.balances[0].amount -= coin_sent.amount;
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
                    TriggerConfiguration::Time {
                        time_interval,
                        mut target_time,
                    } => {
                        let next_trigger_time =
                            get_next_target_time(env.block.time, target_time, time_interval);

                        trigger_store.update(
                            deps.storage,
                            trigger.id.into(),
                            |existing_trigger| match existing_trigger {
                                Some(trigger) => {
                                    target_time = next_trigger_time;
                                    Ok(trigger)
                                }
                                None => Err(ContractError::CustomError {
                                    val: format!("could not trigger with id: {}", trigger.id),
                                }),
                            },
                        )?;
                    }
                    _ => panic!("should be a time based trigger"),
                }

                save_event(
                    deps.storage,
                    EventBuilder::new(
                        vault.id,
                        env.block,
                        EventData::VaultExecutionCompleted {
                            sent: coin_sent.clone(),
                            received: coin_received.clone(),
                        },
                    ),
                )?;

                attributes.push(Attribute::new("status", "success"));
            }
        },
        cosmwasm_std::SubMsgResult::Err(e) => {
            save_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block.to_owned(),
                    EventData::VaultExecutionSkipped {
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
                TriggerConfiguration::Time {
                    time_interval,
                    mut target_time,
                } => {
                    let next_trigger_time =
                        get_next_target_time(env.block.time, target_time, time_interval);

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
        }
    };

    CACHE.remove(deps.storage);

    Ok(Response::new()
        .add_attribute("method", "after_execute_vault_by_address_and_id")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_attributes(attributes)
        .add_messages(messages))
}
