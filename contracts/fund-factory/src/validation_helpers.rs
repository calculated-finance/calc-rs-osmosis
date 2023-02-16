use base::ContractError;
use cosmwasm_std::{Addr, Storage};
use fund_router::msg::ConfigResponse;

use crate::state::config::get_config;

pub fn assert_sender_is_admin(
    storage: &mut dyn Storage,
    sender: Addr,
) -> Result<(), ContractError> {
    let config = get_config(storage)?;
    if sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn assert_sender_is_router_owner_or_admin(
    storage: &mut dyn Storage,
    sender: Addr,
    router_config: &ConfigResponse,
) -> Result<(), ContractError> {
    if sender != router_config.config.owner {
        assert_sender_is_admin(storage, sender)?;
    }
    Ok(())
}
