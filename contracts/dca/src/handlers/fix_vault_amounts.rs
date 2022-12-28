use crate::{
    contract::{AFTER_BANK_SWAP_REPLY_ID, AFTER_Z_DELEGATION_REPLY_ID},
    error::ContractError,
    state::{
        cache::{Cache, CACHE},
        config::{get_config, get_custom_fee},
        data_fixes::{save_data_fix, DataFixBuilder, DataFixData},
        vaults::{get_vault, update_vault},
    },
    types::vault::Vault,
    validation_helpers::assert_sender_is_admin,
};
use base::{helpers::math_helpers::checked_mul, vaults::vault::PostExecutionAction};
use cosmwasm_std::{
    to_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use staking_router::msg::ExecuteMsg as StakingRouterExecuteMsg;
use std::cmp::min;

use super::get_events_by_resource_id::get_events_by_resource_id;

pub fn fix_vault_amounts(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_id: Uint128,
    expected_swapped: Coin,
    expected_received: Coin,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    let mut messages: Vec<CosmosMsg> = Vec::new();
    let mut sub_msgs: Vec<SubMsg> = Vec::new();

    let vault = get_vault(deps.storage, vault_id)?;

    if vault.get_swap_denom() != expected_swapped.denom {
        return Err(ContractError::CustomError {
            val: "Expected swapped denom does not match vault swap denom".to_string(),
        });
    }

    if vault.get_receive_denom() != expected_received.denom {
        return Err(ContractError::CustomError {
            val: "Expected received denom does not match vault received denom".to_string(),
        });
    }

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, Some(1000))?;

    let total_fees_taken_before_fix =
        events
            .events
            .iter()
            .fold(Uint128::zero(), |acc, event| match &event.data {
                base::events::event::EventData::DcaVaultExecutionCompleted { fee, .. } => {
                    acc.checked_add(fee.amount).unwrap()
                }
                _ => acc,
            });

    if expected_received.amount > vault.received_amount.amount + total_fees_taken_before_fix {
        let coin_received = Coin::new(
            (expected_received.amount.clone()
                - (vault.received_amount.amount + total_fees_taken_before_fix))
                .into(),
            expected_received.denom.clone(),
        );

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
                    PostExecutionAction::Send => messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: destination.address.to_string(),
                        amount: vec![allocation_amount],
                    })),
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

        update_vault(
            deps.storage,
            vault_id,
            |stored_value: Option<Vault>| -> StdResult<Vault> {
                match stored_value {
                    Some(mut existing_vault) => {
                        existing_vault.swapped_amount = expected_swapped.clone();
                        existing_vault.received_amount = Coin::new(
                            (existing_vault.received_amount.amount + total_after_total_fee).into(),
                            vault.get_receive_denom(),
                        );
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
    } else {
        update_vault(
            deps.storage,
            vault_id,
            |stored_value: Option<Vault>| -> StdResult<Vault> {
                match stored_value {
                    Some(mut existing_vault) => {
                        existing_vault.swapped_amount = expected_swapped.clone();
                        existing_vault.received_amount = Coin::new(
                            (expected_received.amount - total_fees_taken_before_fix).into(),
                            vault.get_receive_denom(),
                        );
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

    save_data_fix(
        deps.storage,
        DataFixBuilder::new(
            vault.id,
            env.block,
            DataFixData::VaultAmounts {
                old_swapped: vault.swapped_amount,
                old_received: vault.received_amount,
                new_swapped: expected_swapped,
                new_received: expected_received,
            },
        ),
    )?;

    CACHE.save(
        deps.storage,
        &Cache {
            vault_id: vault.id,
            owner: vault.owner.clone(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "fix_vault_amounts")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_attribute("result", "updated")
        .add_messages(messages)
        .add_submessages(sub_msgs))
}
