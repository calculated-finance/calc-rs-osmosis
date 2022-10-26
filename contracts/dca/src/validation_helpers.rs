use crate::error::ContractError;
use crate::state::CONFIG;
use crate::vault::Vault;
use base::vaults::vault::{Destination, PostExecutionAction, VaultStatus};
use base::{pair::Pair, vaults::vault::PositionType};
use cosmwasm_std::{Addr, Coin, Decimal, Deps, Storage, Timestamp, Uint128};

pub fn assert_exactly_one_asset(funds: Vec<Coin>) -> Result<(), ContractError> {
    if funds.is_empty() || funds.len() > 1 {
        return Err(ContractError::CustomError {
            val: format!("received {} denoms but required exactly 1", funds.len()),
        });
    }
    Ok(())
}

pub fn assert_sender_is_admin(
    storage: &mut dyn Storage,
    sender: Addr,
) -> Result<(), ContractError> {
    let config = CONFIG.load(storage)?;
    if sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn asset_sender_is_vault_owner(vault_owner: Addr, sender: Addr) -> Result<(), ContractError> {
    if sender != vault_owner {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn assert_sender_is_admin_or_vault_owner(
    storage: &mut dyn Storage,
    vault_owner: Addr,
    sender: Addr,
) -> Result<(), ContractError> {
    let config = CONFIG.load(storage)?;
    if sender != config.admin && sender != vault_owner {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn assert_vault_is_not_cancelled(vault: &Vault) -> Result<(), ContractError> {
    if vault.status == VaultStatus::Cancelled {
        return Err(ContractError::CustomError {
            val: "vault is already cancelled".to_string(),
        });
    }
    Ok(())
}

pub fn assert_swap_amount_is_less_than_or_equal_to_balance(
    swap_amount: Uint128,
    starting_balance: Coin,
) -> Result<(), ContractError> {
    if starting_balance.amount < swap_amount {
        return Err(ContractError::CustomError {
            val: format!(
                "swap amount of {} is less than the starting balance {}",
                swap_amount, starting_balance.amount
            ),
        });
    }
    Ok(())
}

pub fn assert_denom_matches_pair_denom(
    pair: Pair,
    funds: Vec<Coin>,
    position_type: PositionType,
) -> Result<(), ContractError> {
    match position_type {
        PositionType::Enter => {
            if funds[0].denom.clone() != pair.quote_denom {
                return Err(ContractError::CustomError {
                    val: format!(
                        "received asset with denom: {}, but needed {}",
                        funds[0].denom, pair.quote_denom
                    ),
                });
            }
            Ok(())
        }
        PositionType::Exit => {
            if funds[0].denom.clone() != pair.base_denom {
                return Err(ContractError::CustomError {
                    val: format!(
                        "received asset with denom: {}, but needed {}",
                        funds[0].denom, pair.base_denom
                    ),
                });
            }
            Ok(())
        }
    }
}

pub fn assert_target_start_time_is_in_future(
    current_time: Timestamp,
    target_start_time: Timestamp,
) -> Result<(), ContractError> {
    if current_time.seconds().gt(&target_start_time.seconds()) {
        return Err(ContractError::CustomError {
            val: String::from("target_start_time_utc_seconds must be some time in the future"),
        });
    }
    Ok(())
}

pub fn assert_target_time_is_in_past(
    current_time: Timestamp,
    target_time: Timestamp,
) -> Result<(), ContractError> {
    if current_time.seconds().lt(&target_time.seconds()) {
        return Err(ContractError::CustomError {
            val: String::from("trigger execution time has not yet elapsed"),
        });
    }
    Ok(())
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
        assert_address_is_valid(deps, destination.address.clone(), "destination".to_string())?;
    }

    Ok(())
}

pub fn assert_destination_validator_addresses_are_valid(
    deps: Deps,
    destinations: &[Destination],
) -> Result<(), ContractError> {
    for destination in destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
    {
        assert_validator_is_valid(deps, destination.address.to_string())?;
    }

    Ok(())
}

pub fn assert_address_is_valid(
    deps: Deps,
    address: Addr,
    label: String,
) -> Result<(), ContractError> {
    match deps.api.addr_validate(&address.to_string()) {
        Ok(_) => Ok(()),
        Err(_) => Err(ContractError::CustomError {
            val: format!("{:?} address {:?} is invalid", label, address),
        }),
    }
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

pub fn assert_page_limit_is_valid(limit: Option<u8>) -> Result<(), ContractError> {
    if limit.unwrap_or(30) > 30 {
        return Err(ContractError::CustomError {
            val: String::from("limit cannot be greater than 30."),
        });
    }

    Ok(())
}

pub fn assert_validator_is_valid(
    deps: Deps,
    validator_address: String,
) -> Result<(), ContractError> {
    let validator = deps
        .querier
        .query_validator(validator_address.clone())
        .unwrap();

    if validator.is_none() {
        return Err(ContractError::CustomError {
            val: format!("validator {} is invalid", validator_address),
        });
    }
    Ok(())
}
