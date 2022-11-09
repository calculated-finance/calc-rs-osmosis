use crate::contract::AFTER_Z_DELEGATION_REPLY_ID;
use crate::error::ContractError;
use crate::state::cache::{CACHE, LIMIT_ORDER_CACHE};
use crate::state::config::{get_config, get_custom_fee};
use crate::state::events::create_event;
use crate::state::triggers::{delete_trigger, save_trigger};
use crate::state::vaults::{get_vault, update_vault};
use crate::types::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::helpers::coin_helpers::add_to_coin;
use base::helpers::math_helpers::checked_mul;
use base::helpers::time_helpers::get_next_target_time;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use base::vaults::vault::{PostExecutionAction, VaultStatus};
use cosmwasm_std::{to_binary, CosmosMsg, Env, StdError, StdResult, SubMsg, Uint128, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Reply, Response};
use staking_router::msg::ExecuteMsg as StakingRouterExecuteMsg;
use std::cmp::min;

pub fn after_fin_limit_order_withdrawn_for_execute_vault(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id.into())?;

    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let mut messages: Vec<CosmosMsg> = Vec::new();
            let mut sub_msgs: Vec<SubMsg> = Vec::new();

            delete_trigger(deps.storage, vault.id)?;

            save_trigger(
                deps.storage,
                Trigger {
                    vault_id: vault.id,
                    configuration: TriggerConfiguration::Time {
                        target_time: get_next_target_time(
                            env.block.time,
                            env.block.time,
                            vault.time_interval.clone(),
                        ),
                    },
                },
            )?;

            let coin_received = Coin {
                denom: vault.get_receive_denom().clone(),
                amount: limit_order_cache.filled,
            };

            let config = get_config(deps.storage)?;

            let fee_percent = match (
                get_custom_fee(deps.storage, vault.get_swap_denom()),
                get_custom_fee(deps.storage, vault.get_receive_denom()),
            ) {
                (Some(swap_denom_fee_percent), Some(receive_denom_fee_percent)) => {
                    min(swap_denom_fee_percent, receive_denom_fee_percent)
                }
                (Some(swap_denom_fee_percent), None) => swap_denom_fee_percent,
                (None, Some(receive_denom_fee_percent)) => receive_denom_fee_percent,
                (None, None) => config.swap_fee_percent,
            };

            let execution_fee = Coin::new(
                checked_mul(coin_received.amount, fee_percent)?.into(),
                &coin_received.denom,
            );

            let total_to_redistribute = coin_received.amount - execution_fee.amount;

            update_vault(
                deps.storage,
                vault.id.into(),
                |stored_value: Option<Vault>| -> StdResult<Vault> {
                    match stored_value {
                        Some(mut existing_vault) => {
                            existing_vault.balance.amount -=
                                limit_order_cache.original_offer_amount;

                            if existing_vault.is_empty() {
                                existing_vault.status = VaultStatus::Inactive
                            }

                            existing_vault.swapped_amount = add_to_coin(
                                existing_vault.swapped_amount,
                                limit_order_cache.original_offer_amount,
                            )?;

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

            // never try to send 0 tokens
            if execution_fee.amount.gt(&Uint128::zero()) {
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.fee_collector.to_string(),
                    amount: vec![execution_fee.clone()],
                }));
            }

            let mut total_automation_fees = Uint128::zero();

            vault.destinations.iter().for_each(|destination| {
                let allocation_amount = checked_mul(total_to_redistribute, destination.allocation)
                    .ok()
                    .expect("amount to redistribute should be a value");

                match destination.action {
                    PostExecutionAction::Send => messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: destination.address.to_string(),
                        amount: vec![Coin::new(allocation_amount.into(), &coin_received.denom)],
                    })),
                    PostExecutionAction::ZDelegate => {
                        // authz delegations use funds from the users wallet so send back to user
                        let delegation_fee =
                            checked_mul(allocation_amount, config.delegation_fee_percent)
                                .expect("amount to be taken should be valid");

                        total_automation_fees = total_automation_fees
                            .checked_add(delegation_fee)
                            .expect("amount to add should be valid")
                            .into();

                        let amount_to_delegate = Coin::new(
                            allocation_amount
                                .checked_sub(delegation_fee)
                                .expect("amount to delegate should be valid")
                                .into(),
                            coin_received.denom.clone(),
                        );

                        if amount_to_delegate.amount.gt(&Uint128::zero()) {
                            messages.push(CosmosMsg::Bank(BankMsg::Send {
                                to_address: vault.owner.to_string(),
                                amount: vec![amount_to_delegate.clone()],
                            }));

                            sub_msgs.push(SubMsg::reply_always(
                                CosmosMsg::Wasm(WasmMsg::Execute {
                                    contract_addr: config.staking_router_address.to_string(),
                                    msg: to_binary(&StakingRouterExecuteMsg::ZDelegate {
                                        delegator_address: vault.owner.clone(),
                                        validator_address: destination.address.clone(),
                                        denom: amount_to_delegate.denom.clone(),
                                        amount: amount_to_delegate.amount.clone(),
                                    })
                                    .unwrap(),
                                    funds: vec![],
                                }),
                                AFTER_Z_DELEGATION_REPLY_ID,
                            ))
                        }
                    }
                }
            });

            if total_automation_fees.gt(&Uint128::zero()) {
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.fee_collector.to_string(),
                    amount: vec![Coin::new(
                        total_automation_fees.into(),
                        coin_received.denom.clone(),
                    )],
                }));
            }

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block,
                    EventData::DcaVaultExecutionCompleted {
                        sent: Coin {
                            denom: vault.get_swap_denom().clone(),
                            amount: limit_order_cache.original_offer_amount,
                        },
                        received: coin_received,
                        fee: execution_fee,
                    },
                ),
            )?;

            Ok(Response::new()
                .add_attribute("method", "fin_limit_order_withdrawn_for_execute_vault")
                .add_attribute("vault_id", vault.id)
                .add_messages(messages)
                .add_submessages(sub_msgs))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to withdraw fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}
