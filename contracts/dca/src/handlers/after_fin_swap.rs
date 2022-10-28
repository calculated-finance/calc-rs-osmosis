use crate::contract::AFTER_Z_DELEGATION_REPLY_ID;
use crate::error::ContractError;
use crate::state::cache::CACHE;
use crate::state::config::CONFIG;
use crate::state::events::create_event;
use crate::state::triggers::{delete_trigger, get_trigger, save_trigger};
use crate::state::vaults::{get_vault, update_vault};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData, ExecutionSkippedReason};
use base::helpers::coin_helpers::add_to_coin;
use base::helpers::math_helpers::checked_mul;
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::helpers::time_helpers::get_next_target_time;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use base::vaults::vault::{PositionType, PostExecutionAction, VaultStatus};
use cosmwasm_std::{to_binary, StdError, StdResult, SubMsg, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Attribute, BankMsg, Coin, CosmosMsg, DepsMut, Env, Reply, Response, Uint128};
use staking_router::msg::ExecuteMsg as StakingRouterExecuteMsg;

pub fn after_fin_swap(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id.into())?;
    let trigger = get_trigger(deps.storage, vault.id.into())?;

    let mut attributes: Vec<Attribute> = Vec::new();
    let mut messages: Vec<CosmosMsg> = Vec::new();
    let mut sub_msgs: Vec<SubMsg> = Vec::new();

    delete_trigger(deps.storage, vault.id)?;

    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_swap_response = reply.result.into_result().unwrap();

            let wasm_trade_event =
                get_flat_map_for_event_type(&fin_swap_response.events, "wasm-trade").unwrap();

            let base_amount = wasm_trade_event["base_amount"].parse::<u128>().unwrap();
            let quote_amount = wasm_trade_event["quote_amount"].parse::<u128>().unwrap();

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
                checked_mul(coin_received.amount, config.fee_percent)?.into(),
                &coin_received.denom,
            );

            // never try to send 0 tokens
            if execution_fee.amount.gt(&Uint128::zero()) {
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.fee_collector.to_string(),
                    amount: vec![execution_fee.clone()],
                }));
            }

            let total_to_redistribute = coin_received.amount - execution_fee.amount;

            vault.destinations.iter().for_each(|destination| {
                let amount = checked_mul(total_to_redistribute, destination.allocation)
                    .ok()
                    .expect("amount to be distributed should be valid");

                match destination.action {
                    PostExecutionAction::Send => messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: destination.address.to_string(),
                        amount: vec![Coin::new(amount.into(), &coin_received.denom)],
                    })),
                    PostExecutionAction::ZDelegate => {
                        // authz delegations use funds from the users wallet so send back to user
                        messages.push(CosmosMsg::Bank(BankMsg::Send {
                            to_address: vault.owner.to_string(),
                            amount: vec![Coin::new(amount.into(), &coin_received.denom)],
                        }));
                        sub_msgs.push(SubMsg::reply_always(
                            CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: config.staking_router_address.to_string(),
                                msg: to_binary(&StakingRouterExecuteMsg::ZDelegate {
                                    delegator_address: vault.owner.clone(),
                                    validator_address: destination.address.clone(),
                                    denom: vault.get_receive_denom(),
                                    amount,
                                })
                                .unwrap(),
                                funds: vec![],
                            }),
                            AFTER_Z_DELEGATION_REPLY_ID,
                        ))
                    }
                }
            });

            update_vault(
                deps.storage,
                vault.id.into(),
                |stored_value: Option<Vault>| -> StdResult<Vault> {
                    match stored_value {
                        Some(mut existing_vault) => {
                            existing_vault.balance.amount -=
                                existing_vault.get_swap_amount().amount;

                            if existing_vault.is_empty() {
                                existing_vault.status = VaultStatus::Inactive;
                            }

                            existing_vault.swapped_amount =
                                add_to_coin(existing_vault.swapped_amount, coin_sent.amount)?;

                            existing_vault.received_amount =
                                add_to_coin(existing_vault.received_amount, total_to_redistribute)?;

                            Ok(existing_vault)
                        }
                        None => Err(StdError::NotFound {
                            kind: format!(
                                "vault for address: {} with id: {}",
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
                _ => panic!("should be a time trigger"),
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
            let execution_skipped_reason = ExecutionSkippedReason::from_fin_swap_error(e);

            if execution_skipped_reason == ExecutionSkippedReason::InsufficientFunds {
                update_vault(
                    deps.storage,
                    vault.id.into(),
                    |existing_vault| -> StdResult<Vault> {
                        match existing_vault {
                            Some(mut existing_vault) => {
                                existing_vault.status = VaultStatus::Inactive;
                                Ok(existing_vault)
                            }
                            None => Err(StdError::NotFound {
                                kind: format!(
                                    "vault for address: {} with id: {}",
                                    vault.owner.clone(),
                                    vault.id
                                ),
                            }),
                        }
                    },
                )?;
            }

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block.to_owned(),
                    EventData::DCAVaultExecutionSkipped {
                        reason: execution_skipped_reason.clone(),
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
                _ => panic!("should be a time trigger"),
            }
        }
    };

    Ok(Response::new()
        .add_attribute("method", "fin_swap_completed")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_attributes(attributes)
        .add_messages(messages)
        .add_submessages(sub_msgs))
}
