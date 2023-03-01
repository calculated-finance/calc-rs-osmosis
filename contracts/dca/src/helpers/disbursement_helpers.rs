use crate::{
    contract::{AFTER_BANK_SWAP_REPLY_ID, AFTER_Z_DELEGATION_REPLY_ID},
    state::config::get_config,
    types::vault::Vault,
};
use base::{
    helpers::{community_pool::create_fund_community_pool_msg, math_helpers::checked_mul},
    vaults::vault::PostExecutionAction,
};
use cosmwasm_std::{
    to_binary, BankMsg, Coin, CosmosMsg, Deps, Env, StdResult, SubMsg, Uint128, WasmMsg,
};
use staking_router::msg::ExecuteMsg;

pub fn get_disbursement_messages(
    deps: Deps,
    vault: &Vault,
    amount_to_disburse: Uint128,
) -> StdResult<Vec<SubMsg>> {
    let config = get_config(deps.storage)?;

    Ok(vault
        .destinations
        .iter()
        .flat_map(|destination| {
            let allocation_amount = Coin::new(
                checked_mul(amount_to_disburse, destination.allocation)
                    .ok()
                    .expect("amount to be distributed should be valid")
                    .into(),
                vault.get_receive_denom().clone(),
            );

            if allocation_amount.amount.gt(&Uint128::zero()) {
                return match destination.action {
                    PostExecutionAction::Send => {
                        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                            to_address: destination.address.to_string(),
                            amount: vec![allocation_amount],
                        }))]
                    }
                    PostExecutionAction::ZDelegate => {
                        vec![
                            SubMsg::reply_on_success(
                                BankMsg::Send {
                                    to_address: vault.owner.to_string(),
                                    amount: vec![allocation_amount.clone()],
                                },
                                AFTER_BANK_SWAP_REPLY_ID,
                            ),
                            SubMsg::reply_always(
                                CosmosMsg::Wasm(WasmMsg::Execute {
                                    contract_addr: config.staking_router_address.to_string(),
                                    msg: to_binary(&ExecuteMsg::ZDelegate {
                                        delegator_address: vault.owner.clone(),
                                        validator_address: destination.address.clone(),
                                        denom: allocation_amount.denom.clone(),
                                        amount: allocation_amount.amount.clone(),
                                    })
                                    .unwrap(),
                                    funds: vec![],
                                }),
                                AFTER_Z_DELEGATION_REPLY_ID,
                            ),
                        ]
                    }
                };
            }

            vec![]
        })
        .collect::<Vec<SubMsg>>())
}

pub fn get_fee_messages(
    deps: Deps,
    env: Env,
    fee_amounts: Vec<Uint128>,
    denom: String,
) -> StdResult<Vec<SubMsg>> {
    let config = get_config(deps.storage)?;

    Ok(config
        .fee_collectors
        .iter()
        .flat_map(|fee_collector| {
            fee_amounts.iter().flat_map(|fee| {
                let fee_allocation = Coin::new(
                    checked_mul(*fee, fee_collector.allocation)
                        .ok()
                        .expect("amount to be distributed should be valid")
                        .into(),
                    denom.clone(),
                );

                if fee_allocation.amount.gt(&Uint128::zero()) {
                    Some(match fee_collector.address.as_str() {
                        "community_pool" => create_fund_community_pool_msg(
                            env.contract.address.to_string(),
                            vec![fee_allocation.clone()],
                        ),
                        _ => SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                            to_address: fee_collector.address.to_string(),
                            amount: vec![fee_allocation],
                        })),
                    })
                } else {
                    None
                }
            })
        })
        .collect::<Vec<SubMsg>>())
}
