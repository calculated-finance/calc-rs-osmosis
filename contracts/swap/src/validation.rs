use crate::state::config::get_config;
use cosmwasm_std::{Addr, Coin, Env, StdError, StdResult, Storage};

pub fn assert_sender_is_admin(storage: &mut dyn Storage, sender: Addr) -> StdResult<()> {
    let config = get_config(storage)?;
    if sender != config.admin {
        return Err(StdError::GenericErr {
            msg: "Unauthorised".to_string(),
        });
    }
    Ok(())
}

pub fn assert_sender_is_contract(sender: &Addr, env: &Env) -> StdResult<()> {
    if sender != &env.contract.address {
        return Err(StdError::GenericErr {
            msg: "Unauthorised".to_string(),
        });
    }
    Ok(())
}

pub fn assert_exactly_one_asset(funds: &Vec<Coin>) -> StdResult<()> {
    if funds.is_empty() || funds.len() > 1 {
        return Err(StdError::GenericErr {
            msg: format!("received {} denoms but required exactly 1", funds.len()),
        });
    }
    Ok(())
}
