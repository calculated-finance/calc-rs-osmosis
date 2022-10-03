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

pub fn validate_number_of_executions(
    starting_balance: Coin,
    swap_amount: Uint128,
    number_of_executions: u16,
) -> Result<(), ContractError> {
    let number_of_primary_swaps = starting_balance.amount / swap_amount;
    let number_of_remaining_swaps = if starting_balance.amount % swap_amount != Uint128::zero() {
        Uint128::new(1)
    } else {
        Uint128::zero()
    }; // if there is any asset remaining, we need to do one more swap

    if number_of_primary_swaps + number_of_remaining_swaps == Uint128::from(number_of_executions) {
        Ok(())
    } else {
        Err(ContractError::CustomError {
            val: format!(
                "invalid number of executions: {}, swap amount: {}, starting balance: {}",
                number_of_executions, swap_amount, starting_balance.amount
            ),
        })
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
