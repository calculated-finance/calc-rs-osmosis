use crate::error::ContractError;
use crate::state::config::get_config;
use crate::state::pairs::find_pair;
use crate::types::destination::Destination;
use crate::types::fee_collector::FeeCollector;
use crate::types::pair::Pair;
use crate::types::performance_assessment_strategy::PerformanceAssessmentStrategyParams;
use crate::types::swap_adjustment_strategy::SwapAdjustmentStrategyParams;
use crate::types::time_interval::TimeInterval;
use crate::types::vault::{Vault, VaultStatus};
use cosmwasm_std::{Addr, Coin, Decimal, Deps, Env, QuerierWrapper, Storage, Timestamp, Uint128};

use super::routes::calculate_route;

pub fn assert_exactly_one_asset(funds: Vec<Coin>) -> Result<(), ContractError> {
    if funds.is_empty() || funds.len() > 1 {
        return Err(ContractError::CustomError {
            val: format!("received {} denoms but required exactly 1", funds.len()),
        });
    }
    Ok(())
}

pub fn assert_contract_is_not_paused(storage: &mut dyn Storage) -> Result<(), ContractError> {
    let config = get_config(storage)?;
    if config.paused {
        return Err(ContractError::CustomError {
            val: "contract is paused".to_string(),
        });
    }
    Ok(())
}

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

pub fn assert_sender_is_executor(
    storage: &mut dyn Storage,
    env: &Env,
    sender: &Addr,
) -> Result<(), ContractError> {
    let config = get_config(storage)?;
    if !config.executors.contains(sender)
        && sender != config.admin
        && sender != env.contract.address
    {
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
    let config = get_config(storage)?;
    if sender != config.admin && sender != vault_owner {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn assert_sender_is_contract_or_admin(
    storage: &mut dyn Storage,
    sender: &Addr,
    env: &Env,
) -> Result<(), ContractError> {
    let config = get_config(storage)?;
    if sender != config.admin && sender != env.contract.address {
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

pub fn assert_swap_amount_is_greater_than_50000(swap_amount: Uint128) -> Result<(), ContractError> {
    if swap_amount <= Uint128::from(50000u128) {
        return Err(ContractError::CustomError {
            val: String::from("swap amount must be greater than 50000"),
        });
    }
    Ok(())
}

pub fn assert_deposited_denom_matches_send_denom(
    deposit_denom: String,
    send_denom: String,
) -> Result<(), ContractError> {
    if deposit_denom != send_denom {
        return Err(ContractError::CustomError {
            val: format!(
                "received asset with denom {}, but needed {}",
                deposit_denom, send_denom
            ),
        });
    }
    Ok(())
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

pub fn assert_time_interval_is_valid(interval: &TimeInterval) -> Result<(), ContractError> {
    if let TimeInterval::Custom { seconds } = interval {
        if *seconds < 60 {
            return Err(ContractError::CustomError {
                val: String::from("custom time interval must be at least 60 seconds"),
            });
        }
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

pub fn assert_destination_callback_addresses_are_valid(
    deps: Deps,
    destinations: &[Destination],
) -> Result<(), ContractError> {
    destinations.iter().for_each(|destination| {
        assert_address_is_valid(deps, destination.address.clone(), "destination").unwrap();
    });
    Ok(())
}

pub fn assert_fee_collector_addresses_are_valid(
    deps: Deps,
    fee_collectors: &[FeeCollector],
) -> Result<(), ContractError> {
    for fee_collector in fee_collectors {
        assert_address_is_valid(
            deps,
            Addr::unchecked(fee_collector.address.clone()),
            "fee collector",
        )?;
    }
    Ok(())
}

pub fn assert_address_is_valid(
    deps: Deps,
    address: Addr,
    label: &str,
) -> Result<(), ContractError> {
    deps.api
        .addr_validate(address.as_ref())
        .map(|_| ())
        .map_err(|_| ContractError::CustomError {
            val: format!("{} address {} is invalid", label, address),
        })
}

pub fn assert_addresses_are_valid(
    deps: Deps,
    addresses: &[Addr],
    label: &str,
) -> Result<(), ContractError> {
    addresses
        .iter()
        .try_for_each(|address| assert_address_is_valid(deps, address.clone(), label))
}

pub fn assert_pair_exists_for_denoms(
    deps: Deps,
    swap_denom: String,
    target_denom: String,
) -> Result<(), ContractError> {
    find_pair(deps.storage, &[swap_denom.clone(), target_denom.clone()])
        .map(|_| ())
        .map_err(|_| ContractError::CustomError {
            val: format!("swapping {} to {} not supported", swap_denom, target_denom),
        })
}

pub fn assert_swap_adjusment_and_performance_assessment_strategies_are_compatible(
    swap_adjustment_strategy_params: &Option<SwapAdjustmentStrategyParams>,
    performance_assessment_strategy_params: &Option<PerformanceAssessmentStrategyParams>,
) -> Result<(), ContractError> {
    match swap_adjustment_strategy_params {
        Some(SwapAdjustmentStrategyParams::RiskWeightedAverage { .. }) => {
            match performance_assessment_strategy_params {
                Some(PerformanceAssessmentStrategyParams::CompareToStandardDca) => Ok(()),
                None => Err(ContractError::CustomError {
                    val: "incompatible swap adjustment and performance assessment strategies"
                        .to_string(),
                }),
            }
        }
        Some(SwapAdjustmentStrategyParams::WeightedScale { .. }) => {
            match performance_assessment_strategy_params {
                Some(PerformanceAssessmentStrategyParams::CompareToStandardDca) => {
                    Err(ContractError::CustomError {
                        val: "incompatible swap adjustment and performance assessment strategies"
                            .to_string(),
                    })
                }
                None => Ok(()),
            }
        }
        None => match performance_assessment_strategy_params {
            Some(_) => Err(ContractError::CustomError {
                val: "incompatible swap adjustment and performance assessment strategies"
                    .to_string(),
            }),
            None => Ok(()),
        },
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

pub fn assert_fee_collector_allocations_add_up_to_one(
    fee_collectors: &[FeeCollector],
) -> Result<(), ContractError> {
    if fee_collectors
        .iter()
        .fold(Decimal::zero(), |acc, fee_collector| {
            acc.checked_add(fee_collector.allocation).unwrap()
        })
        != Decimal::percent(100)
    {
        return Err(ContractError::CustomError {
            val: String::from("fee collector allocations must add up to 1"),
        });
    }
    Ok(())
}

pub fn assert_risk_weighted_average_escrow_level_is_less_than_100_percent(
    risk_weighted_average_escrow_level: Decimal,
) -> Result<(), ContractError> {
    if risk_weighted_average_escrow_level > Decimal::percent(100) {
        return Err(ContractError::CustomError {
            val: "risk_weighted_average_escrow_level cannot be greater than 100%".to_string(),
        });
    }
    Ok(())
}

pub fn assert_no_destination_allocations_are_zero(
    destinations: &[Destination],
) -> Result<(), ContractError> {
    if destinations.iter().any(|d| d.allocation.is_zero()) {
        return Err(ContractError::CustomError {
            val: String::from("all destination allocations must be greater than 0"),
        });
    }
    Ok(())
}

pub fn assert_page_limit_is_valid(
    storage: &dyn Storage,
    limit: Option<u16>,
) -> Result<(), ContractError> {
    let config = get_config(storage)?;
    if limit.unwrap_or(30) > config.page_limit {
        return Err(ContractError::CustomError {
            val: format!("limit cannot be greater than {}.", config.page_limit),
        });
    }
    Ok(())
}

pub fn assert_validator_is_valid(
    deps: Deps,
    validator_address: String,
) -> Result<(), ContractError> {
    let validator = deps.querier.query_validator(validator_address.clone()).ok();

    if validator.is_none() {
        return Err(ContractError::CustomError {
            val: format!("validator {} is invalid", validator_address),
        });
    }
    Ok(())
}

pub fn assert_denom_is_bond_denom(denom: String) -> Result<(), ContractError> {
    if denom != *"uosmo" {
        return Err(ContractError::CustomError {
            val: format!("{} is not the bond denomination", denom),
        });
    }
    Ok(())
}

pub fn assert_label_is_no_longer_than_100_characters(
    label: &Option<String>,
) -> Result<(), ContractError> {
    if let Some(label) = label.clone() {
        if label.len() > 100 {
            return Err(ContractError::CustomError {
                val: "Vault label cannot be longer than 100 characters".to_string(),
            });
        }
    }
    Ok(())
}

pub fn assert_route_matches_denoms(
    querier: &QuerierWrapper,
    pair: &Pair,
) -> Result<(), ContractError> {
    for denom in pair.denoms() {
        calculate_route(querier, pair, denom.clone())?;
    }
    Ok(())
}

pub fn assert_route_not_empty(route: Vec<u64>) -> Result<(), ContractError> {
    if route.is_empty() {
        return Err(ContractError::CustomError {
            val: "Swap route must not be empty".to_string(),
        });
    }
    Ok(())
}
