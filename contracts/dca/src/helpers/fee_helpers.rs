use std::cmp::min;

use crate::{
    state::config::{get_config, get_custom_fee, FeeCollector},
    types::vault::Vault,
};
use base::{
    helpers::{community_pool::create_fund_community_pool_msg, math_helpers::checked_mul},
    vaults::vault::PostExecutionAction,
};
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, StdResult, SubMsg, Uint128,
};

pub fn get_fee_messages(
    deps: Deps,
    env: Env,
    fee_amounts: Vec<Uint128>,
    denom: String,
    skip_community_pool: bool,
) -> StdResult<Vec<SubMsg>> {
    let config = get_config(deps.storage)?;

    let fee_collectors = config
        .fee_collectors
        .iter()
        .flat_map(|fee_collector| {
            if skip_community_pool && fee_collector.address == "community_pool" {
                return None;
            }
            return Some(FeeCollector {
                address: fee_collector.address.clone(),
                allocation: if skip_community_pool {
                    let community_pool_allocation = config
                        .fee_collectors
                        .iter()
                        .find(|fee_collector| fee_collector.address == "community_pool")
                        .map_or(Decimal::zero(), |community_pool| community_pool.allocation);
                    fee_collector.allocation / (Decimal::one() - community_pool_allocation)
                } else {
                    fee_collector.allocation
                },
            });
        })
        .collect::<Vec<FeeCollector>>();

    Ok(fee_collectors
        .iter()
        .flat_map(|fee_collector| {
            fee_amounts.iter().flat_map(|fee| {
                let fee_allocation = Coin::new(
                    checked_mul(*fee, fee_collector.allocation)
                        .ok()
                        .expect("amount to be distributed should be valid")
                        .into(),
                    denom.clone(),
                );

                if fee_allocation.amount.gt(&Uint128::zero()) {
                    match fee_collector.address.as_str() {
                        "community_pool" => {
                            if skip_community_pool {
                                None
                            } else {
                                Some(create_fund_community_pool_msg(
                                    env.contract.address.to_string(),
                                    vec![fee_allocation.clone()],
                                ))
                            }
                        }
                        _ => Some(SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                            to_address: fee_collector.address.to_string(),
                            amount: vec![fee_allocation],
                        }))),
                    }
                } else {
                    None
                }
            })
        })
        .collect::<Vec<SubMsg>>())
}

pub fn get_delegation_fee_rate(deps: &DepsMut, vault: &Vault) -> StdResult<Decimal> {
    let config = get_config(deps.storage)?;

    Ok(config.delegation_fee_percent.checked_mul(
        vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum(),
    )?)
}

pub fn get_swap_fee_rate(deps: &DepsMut, vault: &Vault) -> StdResult<Decimal> {
    let config = get_config(deps.storage)?;

    Ok(
        match (
            get_custom_fee(deps.storage, vault.get_swap_denom()),
            get_custom_fee(deps.storage, vault.get_receive_denom()),
        ) {
            (Some(swap_denom_fee_percent), Some(receive_denom_fee_percent)) => {
                min(swap_denom_fee_percent, receive_denom_fee_percent)
            }
            (Some(swap_denom_fee_percent), None) => swap_denom_fee_percent,
            (None, Some(receive_denom_fee_percent)) => receive_denom_fee_percent,
            (None, None) => config.swap_fee_percent,
        },
    )
}

pub fn get_dca_plus_performance_fee(vault: &Vault, current_price: Decimal) -> StdResult<Coin> {
    let dca_plus_config = vault
        .dca_plus_config
        .clone()
        .expect("Only DCA plus vaults should try to get fee");

    let dca_plus_total_value = dca_plus_config.total_deposit.amount - vault.swapped_amount.amount
        + vault.received_amount.amount * current_price;

    let standard_dca_total_value = dca_plus_config.total_deposit.amount
        - dca_plus_config.standard_dca_swapped_amount.amount
        + dca_plus_config.standard_dca_received_amount.amount * current_price;

    if standard_dca_total_value > dca_plus_total_value {
        return Ok(Coin {
            denom: vault.get_swap_denom(),
            amount: Uint128::zero(),
        });
    }

    let value_difference_in_terms_of_receive_denom =
        (dca_plus_total_value - standard_dca_total_value) * (Decimal::one() / current_price);

    let fee = value_difference_in_terms_of_receive_denom * Decimal::percent(20);

    Ok(Coin {
        denom: vault.get_swap_denom(),
        amount: min(fee, dca_plus_config.escrowed_balance.amount),
    })
}
