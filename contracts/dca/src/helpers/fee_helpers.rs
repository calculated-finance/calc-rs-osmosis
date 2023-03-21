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
            denom: vault.get_receive_denom(),
            amount: Uint128::zero(),
        });
    }

    let value_difference_in_terms_of_receive_denom =
        (dca_plus_total_value - standard_dca_total_value) * (Decimal::one() / current_price);

    let fee = value_difference_in_terms_of_receive_denom * Decimal::percent(20);

    Ok(Coin {
        denom: vault.get_receive_denom(),
        amount: min(fee, dca_plus_config.escrowed_balance.amount),
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::TEN,
        helpers::fee_helpers::get_dca_plus_performance_fee,
        types::{dca_plus_config::DcaPlusConfig, vault::Vault},
    };
    use base::{pair::Pair, triggers::trigger::TimeInterval, vaults::vault::VaultStatus};
    use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
    use std::str::FromStr;

    fn get_vault(
        total_deposit: Uint128,
        swapped_amount: Uint128,
        standard_dca_swapped_amount: Uint128,
        received_amount: Uint128,
        standard_dca_received_amount: Uint128,
    ) -> Vault {
        let escrow_level = Decimal::percent(5);

        Vault {
            balance: Coin {
                denom: "swap_denom".to_string(),
                amount: total_deposit - swapped_amount,
            },
            swapped_amount: Coin {
                denom: "swap_denom".to_string(),
                amount: swapped_amount,
            },
            received_amount: Coin {
                denom: "receive_denom".to_string(),
                amount: received_amount,
            },
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(total_deposit.into(), "swap_denom".to_string()),
                standard_dca_swapped_amount: Coin::new(
                    standard_dca_swapped_amount.into(),
                    "swap_denom".to_string(),
                ),
                standard_dca_received_amount: Coin::new(
                    standard_dca_received_amount.into(),
                    "receive_denom".to_string(),
                ),
                escrowed_balance: Coin::new(
                    (received_amount * escrow_level).into(),
                    "denom".to_string(),
                ),
                model_id: 30,
                escrow_level,
            }),
            id: Uint128::one(),
            created_at: Timestamp::from_seconds(10000000),
            owner: Addr::unchecked("owner"),
            label: None,
            destinations: vec![],
            status: VaultStatus::Active,
            pair: Pair {
                address: Addr::unchecked("pair"),
                base_denom: "receive_denom".to_string(),
                quote_denom: "swap_denom".to_string(),
            },
            swap_amount: swapped_amount / Uint128::new(2),
            slippage_tolerance: None,
            minimum_receive_amount: None,
            time_interval: TimeInterval::Daily,
            started_at: None,
            trigger: None,
        }
    }

    fn assert_fee_amount(
        total_deposit: Uint128,
        swapped_amount: Uint128,
        standard_dca_swapped_amount: Uint128,
        received_amount: Uint128,
        standard_dca_received_amount: Uint128,
        current_price: Decimal,
        expected_fee: Uint128,
    ) {
        let vault = get_vault(
            total_deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
        );

        let fee = get_dca_plus_performance_fee(&vault, current_price).unwrap();
        assert_eq!(fee.amount, expected_fee);
    }

    #[test]
    fn non_zero_fee_is_in_vault_receive_denom() {
        let vault = get_vault(TEN, TEN, TEN, TEN + TEN, TEN);

        let fee = get_dca_plus_performance_fee(&vault, Decimal::one()).unwrap();
        assert_eq!(fee.denom, vault.get_receive_denom());
    }

    #[test]
    fn zero_fee_is_in_vault_receive_denom() {
        let vault = get_vault(TEN, TEN, TEN, TEN, TEN);

        let fee = get_dca_plus_performance_fee(&vault, Decimal::one()).unwrap();
        assert_eq!(fee.denom, vault.get_receive_denom());
    }

    #[test]
    fn fee_is_zero_when_performance_is_even() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1.0").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_above_zero_when_less_swapped_and_price_dropped() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(900);
        let received_amount = Uint128::new(900);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.9").unwrap();
        let expected_fee = Uint128::new(2);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_equal_to_escrow_when_less_swapped_and_price_dropped_significantly() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(900);
        let received_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.2").unwrap();
        let expected_fee = Uint128::new(50);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_zero_when_more_swapped_and_price_dropped() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1100);
        let received_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.9").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_above_zero_when_more_swapped_and_price_increased() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1100);
        let received_amount = Uint128::new(1100);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("2").unwrap();
        let expected_fee = Uint128::new(10);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_equal_to_escrow_when_same_amount_swapped_and_more_received() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(2000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1").unwrap();
        let expected_fee = Uint128::new(100);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_zero_when_less_swapped_and_price_increased() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(900);
        let received_amount = Uint128::new(900);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1.1").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }
}
