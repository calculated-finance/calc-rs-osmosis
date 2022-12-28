use crate::contract::{
    AFTER_BANK_SWAP_REPLY_ID, AFTER_FIN_SWAP_REPLY_ID, AFTER_Z_DELEGATION_REPLY_ID,
};
use crate::error::ContractError;
use crate::state::cache::{SwapCache, CACHE, LIMIT_ORDER_CACHE, SWAP_CACHE};
use crate::state::config::{get_config, get_custom_fee};
use crate::state::events::create_event;
use crate::state::fin_limit_order_change_timestamp::FIN_LIMIT_ORDER_CHANGE_TIMESTAMP;
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
use fin_helpers::swaps::create_fin_swap_message;
use staking_router::msg::ExecuteMsg as StakingRouterExecuteMsg;
use std::cmp::min;

pub fn after_fin_limit_order_withdrawn_for_execute_vault(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id.into())?;

    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let mut messages: Vec<CosmosMsg> = Vec::new();
            let mut sub_msgs: Vec<SubMsg> = Vec::new();

            let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;

            let receive_denom_balance = &deps
                .querier
                .query_balance(&env.contract.address, &vault.get_receive_denom())?;

            let withdrawn_amount = receive_denom_balance
                .amount
                .checked_sub(limit_order_cache.receive_denom_balance.amount)
                .expect("withdrawn amount");

            let coin_received = Coin {
                denom: vault.get_receive_denom().clone(),
                amount: withdrawn_amount,
            };

            let config = get_config(deps.storage)?;

            let fin_limit_order_change_timestamp =
                FIN_LIMIT_ORDER_CHANGE_TIMESTAMP.may_load(deps.storage)?;

            let is_new_fin_limit_order = fin_limit_order_change_timestamp.is_some()
                && limit_order_cache.created_at > fin_limit_order_change_timestamp.unwrap();

            if is_new_fin_limit_order {
                if coin_received.amount.gt(&Uint128::zero()) {
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.fee_collector.to_string(),
                        amount: vec![coin_received.clone()],
                    }));
                }

                SWAP_CACHE.save(
                    deps.storage,
                    &SwapCache {
                        swap_denom_balance: deps
                            .querier
                            .query_balance(&env.contract.address, &vault.get_swap_denom())?,
                        receive_denom_balance: Coin::new(
                            (deps
                                .querier
                                .query_balance(&env.contract.address, &vault.get_receive_denom())?
                                .amount
                                - withdrawn_amount)
                                .into(),
                            vault.get_receive_denom().clone(),
                        ),
                    },
                )?;

                sub_msgs.push(create_fin_swap_message(
                    deps.querier,
                    vault.pair.address.clone(),
                    vault.get_swap_amount(),
                    vault.get_position_type(),
                    vault.slippage_tolerance,
                    AFTER_FIN_SWAP_REPLY_ID,
                ));
            } else {
                delete_trigger(deps.storage, vault.id)?;

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

                let automation_fee_rate = config.delegation_fee_percent.checked_mul(
                    vault
                        .destinations
                        .iter()
                        .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
                        .map(|destination| destination.allocation)
                        .sum(),
                )?;

                let swap_fee = checked_mul(coin_received.amount, fee_percent)?;
                let total_after_swap_fee = coin_received.amount - swap_fee;
                let automation_fee = checked_mul(total_after_swap_fee, automation_fee_rate)?;

                if swap_fee.gt(&Uint128::zero()) {
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.fee_collector.to_string(),
                        amount: vec![Coin::new(swap_fee.into(), coin_received.denom.clone())],
                    }));
                }

                if automation_fee.gt(&Uint128::zero()) {
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.fee_collector.to_string(),
                        amount: vec![Coin::new(
                            automation_fee.into(),
                            coin_received.denom.clone(),
                        )],
                    }));
                }

                let total_fee = swap_fee + automation_fee;
                let total_after_total_fee = coin_received.amount - total_fee;

                vault.destinations.iter().for_each(|destination| {
                    let allocation_amount = Coin::new(
                        checked_mul(total_after_total_fee, destination.allocation)
                            .ok()
                            .expect("amount to be distributed should be valid")
                            .into(),
                        coin_received.denom.clone(),
                    );

                    if allocation_amount.amount.gt(&Uint128::zero()) {
                        match destination.action {
                            PostExecutionAction::Send => {
                                messages.push(CosmosMsg::Bank(BankMsg::Send {
                                    to_address: destination.address.to_string(),
                                    amount: vec![allocation_amount],
                                }))
                            }
                            PostExecutionAction::ZDelegate => {
                                sub_msgs.push(SubMsg::reply_on_success(
                                    BankMsg::Send {
                                        to_address: vault.owner.to_string(),
                                        amount: vec![allocation_amount.clone()],
                                    },
                                    AFTER_BANK_SWAP_REPLY_ID,
                                ));
                                sub_msgs.push(SubMsg::reply_always(
                                    CosmosMsg::Wasm(WasmMsg::Execute {
                                        contract_addr: config.staking_router_address.to_string(),
                                        msg: to_binary(&StakingRouterExecuteMsg::ZDelegate {
                                            delegator_address: vault.owner.clone(),
                                            validator_address: destination.address.clone(),
                                            denom: allocation_amount.denom.clone(),
                                            amount: allocation_amount.amount.clone(),
                                        })
                                        .unwrap(),
                                        funds: vec![],
                                    }),
                                    AFTER_Z_DELEGATION_REPLY_ID,
                                ));
                            }
                        }
                    }
                });

                let updated_vault = update_vault(
                    deps.storage,
                    vault.id.into(),
                    |stored_value: Option<Vault>| -> StdResult<Vault> {
                        match stored_value {
                            Some(mut existing_vault) => {
                                existing_vault.balance.amount -=
                                    limit_order_cache.original_offer_amount;

                                if !existing_vault.has_sufficient_funds() {
                                    existing_vault.status = VaultStatus::Inactive
                                }

                                existing_vault.swapped_amount = add_to_coin(
                                    existing_vault.swapped_amount,
                                    limit_order_cache.original_offer_amount,
                                )?;

                                existing_vault.received_amount = add_to_coin(
                                    existing_vault.received_amount,
                                    total_after_total_fee,
                                )?;

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

                if updated_vault.is_active() {
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
                            received: coin_received.clone(),
                            fee: Coin::new(total_fee.into(), coin_received.denom),
                        },
                    ),
                )?;
            }

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
