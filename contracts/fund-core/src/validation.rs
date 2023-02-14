use crate::{contract::ContractResult, state::get_config};
use base::ContractError;
use cosmwasm_std::{Addr, Storage};

pub fn assert_sender_is_router(store: &dyn Storage, sender: Addr) -> ContractResult<()> {
    if sender != get_config(store)?.router {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}
