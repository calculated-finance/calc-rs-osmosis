use crate::{
    contract::AFTER_Z_DELEGATION_REPLY_ID,
    msg::ExecuteMsg,
    state::config::get_config,
    types::{post_execution_action::PostExecutionAction, vault::Vault},
};
use base::helpers::math_helpers::checked_mul;
use cosmwasm_std::{
    to_binary, BankMsg, Coin, CosmosMsg, Deps, Env, StdResult, SubMsg, Uint128, WasmMsg,
};
use staking_router::msg::ExecuteMsg as StakingRouterExecuteMsg;

pub fn get_disbursement_messages(
    deps: Deps,
    env: &Env,
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
                return match destination.action.clone() {
                    PostExecutionAction::Send => {
                        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                            to_address: destination.address.to_string(),
                            amount: vec![allocation_amount],
                        }))]
                    }
                    PostExecutionAction::ZDelegate => {
                        vec![
                            SubMsg::new(BankMsg::Send {
                                to_address: vault.owner.to_string(),
                                amount: vec![allocation_amount.clone()],
                            }),
                            SubMsg::reply_always(
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
                            ),
                        ]
                    }
                    PostExecutionAction::ZProvideLiquidity { pool_id } => {
                        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: env.contract.address.to_string(),
                            msg: to_binary(&ExecuteMsg::ProvideLiquidity {
                                provider_address: destination.address.clone(),
                                pool_id,
                                // duration,
                            })
                            .unwrap(),
                            funds: vec![allocation_amount.clone()],
                        }))]
                    }
                };
            }

            vec![]
        })
        .collect::<Vec<SubMsg>>())
}
