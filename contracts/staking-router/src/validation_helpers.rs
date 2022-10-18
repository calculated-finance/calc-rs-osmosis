use cosmwasm_std::{Addr, Storage};

use crate::{state::CONFIG, ContractError};

pub fn assert_sender_is_admin(storage: &dyn Storage, sender: Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(storage)?;
    if sender == config.admin {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn assert_sender_is_allowed_z_caller(
    storage: &dyn Storage,
    sender: Addr,
) -> Result<(), ContractError> {
    let config = CONFIG.load(storage)?;

    match config.allowed_z_callers.contains(&sender) {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}
