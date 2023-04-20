use crate::types::vault::Vault;
use cosmwasm_std::{BankMsg, Coin, StdResult, SubMsg, Uint128, WasmMsg};

use super::math::checked_mul;

pub fn get_disbursement_messages(
    vault: &Vault,
    amount_to_disburse: Uint128,
) -> StdResult<Vec<SubMsg>> {
    Ok(vault
        .destinations
        .iter()
        .flat_map(|destination| {
            let allocation_amount = Coin::new(
                checked_mul(amount_to_disburse, destination.allocation)
                    .expect("amount to be distributed should be valid")
                    .into(),
                vault.target_denom.clone(),
            );

            if allocation_amount.amount.gt(&Uint128::zero()) {
                return Some(destination.msg.clone().map_or(
                    SubMsg::new(BankMsg::Send {
                        to_address: destination.address.to_string(),
                        amount: vec![allocation_amount.clone()],
                    }),
                    |msg| {
                        SubMsg::new(WasmMsg::Execute {
                            contract_addr: destination.address.to_string(),
                            msg,
                            funds: vec![allocation_amount],
                        })
                    },
                ));
            }

            None
        })
        .collect::<Vec<SubMsg>>())
}
