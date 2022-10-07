use cosmwasm_std::{Addr, Coin, Deps, Timestamp, Uint128};

use crate::error::ContractError;
use crate::state::CONFIG;

use base::pair::Pair;
use base::vaults::dca_vault::PositionType;

pub fn validate_funds(funds: Vec<Coin>) -> Result<(), ContractError> {
    if !funds.is_empty() {
        Ok(())
    } else {
        Err(ContractError::CustomError {
            val: String::from("no funds were sent"),
        })
    }
}

pub fn validate_sender_is_admin(deps: Deps, sender: Addr) -> Result<(), ContractError> {
    // refactor to just take storage
    let config = CONFIG.load(deps.storage)?;
    if sender == config.admin {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn validate_sender_is_admin_or_vault_owner(
    deps: Deps,
    vault_owner: Addr,
    sender: Addr,
) -> Result<(), ContractError> {
    // refactor to take storage
    let config = CONFIG.load(deps.storage)?;
    if sender == config.admin || sender == vault_owner {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn validate_swap_amount(
    swap_amount: Uint128,
    starting_balance: Coin,
) -> Result<(), ContractError> {
    if starting_balance.amount < swap_amount {
        Err(ContractError::CustomError {
            val: format!(
                "swap amount of {} is less than the starting balance {}",
                swap_amount, starting_balance.amount
            ),
        })
    } else {
        Ok(())
    }
}

pub fn validate_asset_denom_matches_pair_denom(
    pair: Pair,
    funds: Vec<Coin>,
    position_type: PositionType,
) -> Result<(), ContractError> {
    match position_type {
        PositionType::Enter => {
            if funds[0].denom.clone() == pair.quote_denom {
                Ok(())
            } else {
                Err(ContractError::CustomError {
                    val: format!(
                        "received asset with denom: {}, but needed {}",
                        funds[0].denom, pair.quote_denom
                    ),
                })
            }
        }
        PositionType::Exit => {
            if funds[0].denom.clone() == pair.base_denom {
                Ok(())
            } else {
                Err(ContractError::CustomError {
                    val: format!(
                        "received asset with denom: {}, but needed {}",
                        funds[0].denom, pair.base_denom
                    ),
                })
            }
        }
    }
}

pub fn validate_target_start_time(
    current_time: Timestamp,
    target_start_time: Timestamp,
) -> Result<(), ContractError> {
    if target_start_time.seconds().ge(&current_time.seconds()) {
        Ok(())
    } else {
        Err(ContractError::CustomError {
            val: String::from("target_start_time_utc_seconds must be some time in the future"),
        })
    }
}
