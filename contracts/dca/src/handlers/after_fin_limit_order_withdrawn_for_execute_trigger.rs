use crate::contract::AFTER_Z_DELEGATION_REPLY_ID;
use crate::error::ContractError;
use crate::state::{
    create_event, delete_trigger, get_vault, save_trigger, update_vault, CACHE, CONFIG,
    LIMIT_ORDER_CACHE,
};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::helpers::math_helpers::checked_mul;
use base::helpers::time_helpers::get_next_target_time;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use base::vaults::vault::{PostExecutionAction, VaultStatus};
use cosmwasm_std::{to_binary, CosmosMsg, Env, StdError, StdResult, SubMsg, Uint128, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Reply, Response};
use staking_router::msg::ExecuteMsg as StakingRouterExecuteMsg;

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

            update_vault(
                deps.storage,
                vault.id.into(),
                |stored_value: Option<Vault>| -> StdResult<Vault> {
                    match stored_value {
                        Some(mut existing_vault) => {
                            existing_vault.balance.amount -=
                                limit_order_cache.original_offer_amount;

                            if existing_vault.low_funds() {
                                existing_vault.status = VaultStatus::Inactive
                            }

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

            let coin_received = Coin {
                denom: vault.get_receive_denom().clone(),
                amount: limit_order_cache.filled,
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
                    .expect("amount to redistribute should be a value");

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

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block,
                    EventData::DCAVaultExecutionCompleted {
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
