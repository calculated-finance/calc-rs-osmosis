use crate::{
    msg::ExecuteMsg,
    types::{post_execution_action::PostExecutionAction, vault::Vault},
};
use base::helpers::math_helpers::checked_mul;
use cosmwasm_std::{to_binary, BankMsg, Coin, CosmosMsg, Env, StdResult, SubMsg, Uint128, WasmMsg};

pub fn get_disbursement_messages(
    env: &Env,
    vault: &Vault,
    amount_to_disburse: Uint128,
) -> StdResult<Vec<SubMsg>> {
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
                        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: env.contract.address.to_string(),
                            msg: to_binary(&ExecuteMsg::ZDelegate {
                                delegator_address: vault.owner.clone(),
                                validator_address: destination.address.clone(),
                            })
                            .unwrap(),
                            funds: vec![allocation_amount],
                        }))]
                    }
                    PostExecutionAction::ZProvideLiquidity { pool_id, duration } => {
                        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: env.contract.address.to_string(),
                            msg: to_binary(&ExecuteMsg::ZProvideLiquidity {
                                provider_address: destination.address.clone(),
                                pool_id,
                                duration,
                                slippage_tolerance: vault.slippage_tolerance,
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
