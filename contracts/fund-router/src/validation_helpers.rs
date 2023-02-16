use base::ContractError;
use cosmwasm_std::{Addr, Storage};

use crate::state::config::get_config;

pub fn assert_sender_is_factory(
    storage: &mut dyn Storage,
    sender: Addr,
) -> Result<(), ContractError> {
    let config = get_config(storage)?;
    if sender != config.factory {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}
