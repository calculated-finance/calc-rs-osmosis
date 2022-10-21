use crate::error::ContractError;
use crate::state::CONFIG;
use base::vaults::vault::{Destination, PostExecutionAction};
use base::{pair::Pair, vaults::vault::PositionType};
use cosmwasm_std::{Addr, Coin, Decimal, Deps, Timestamp, Uint128};

pub fn assert_exactly_one_asset(funds: Vec<Coin>) -> Result<(), ContractError> {
    if funds.is_empty() || funds.len() > 1 {
        Err(ContractError::CustomError {
            val: format!("received {} denoms but required exactly 1", funds.len()),
        })
    } else {
        Ok(())
    }
}

pub fn assert_sender_is_admin(deps: Deps, sender: Addr) -> Result<(), ContractError> {
    // refactor to just take storage
    let config = CONFIG.load(deps.storage)?;
    if sender == config.admin {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn assert_sender_is_admin_or_vault_owner(
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

pub fn assert_swap_amount_is_less_than_or_equal_to_balance(
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

pub fn assert_denom_matches_pair_denom(
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

pub fn assert_target_start_time_is_in_future(
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

pub fn assert_target_time_is_in_past(
    current_time: Timestamp,
    target_time: Timestamp,
) -> Result<(), ContractError> {
    if target_time.seconds().le(&current_time.seconds()) {
        Ok(())
    } else {
        Err(ContractError::CustomError {
            val: String::from("trigger execution time has not yet elapsed"),
        })
    }
}

pub fn assert_destinations_limit_is_not_breached(
    destinations: &[Destination],
) -> Result<(), ContractError> {
    if destinations.len() > 10 {
        return Err(ContractError::CustomError {
            val: String::from("no more than 10 destinations can be provided"),
        });
    };

    Ok(())
}

pub fn assert_destination_send_addresses_are_valid(
    deps: Deps,
    destinations: &[Destination],
) -> Result<(), ContractError> {
    for destination in destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::Send)
    {
        match deps.api.addr_validate(&destination.address.to_string()) {
            Ok(_) => {}
            Err(_) => {
                return Err(ContractError::CustomError {
                    val: format!("destination address {:?} is invalid", destination.address),
                });
            }
        }
    }

    Ok(())
}

pub fn assert_destination_allocations_add_up_to_one(
    destinations: &[Destination],
) -> Result<(), ContractError> {
    if destinations
        .iter()
        .fold(Decimal::zero(), |acc, destintation| {
            acc.checked_add(destintation.allocation).unwrap()
        })
        != Decimal::percent(100)
    {
        return Err(ContractError::CustomError {
            val: String::from("destination allocations must add up to 1"),
        });
    }

    Ok(())
}
