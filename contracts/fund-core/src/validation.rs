use std::collections::HashMap;

use crate::{contract::ContractResult, state::get_config};
use base::ContractError;
use cosmwasm_std::{Addr, Decimal, Storage};

pub fn assert_sender_is_router(store: &dyn Storage, sender: Addr) -> ContractResult<()> {
    if sender != get_config(store)?.router {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn assert_allocations_sum_to_one(allocations: &HashMap<String, Decimal>) -> ContractResult<()> {
    if allocations.values().sum::<Decimal>() != Decimal::one() {
        return Err(ContractError::CustomError {
            val: "provided allocations must sum to 1".to_string(),
        });
    }
    Ok(())
}
