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
