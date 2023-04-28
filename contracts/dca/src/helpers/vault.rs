use super::{
    coin::add_to,
    fees::{get_delegation_fee_rate, get_swap_fee_rate},
    price::{calculate_slippage, query_belief_price, query_price},
    time::get_total_execution_duration,
};
use crate::{
    state::{
        events::create_event, pairs::find_pair, swap_adjustments::get_swap_adjustment,
        vaults::update_vault,
    },
    types::{
        event::{EventBuilder, EventData, ExecutionSkippedReason},
        performance_assessment_strategy::PerformanceAssessmentStrategy,
        position_type::PositionType,
        swap_adjustment_strategy::SwapAdjustmentStrategy,
        time_interval::TimeInterval,
        vault::Vault,
    },
};
use cosmwasm_std::{
    Coin, Decimal, Deps, Env, QuerierWrapper, Response, StdResult, Storage, Timestamp, Uint128,
};
use std::cmp::min;

pub fn get_position_type(deps: &Deps, vault: &Vault) -> StdResult<PositionType> {
    let pair = find_pair(deps.storage, &vault.denoms())?;
    Ok(pair.position_type(vault.get_swap_denom()))
}

pub fn get_swap_amount(deps: &Deps, env: &Env, vault: &Vault) -> StdResult<Coin> {
    let swap_adjustment = match vault.swap_adjustment_strategy.clone() {
        Some(SwapAdjustmentStrategy::WeightedScale {
            base_receive_amount,
            multiplier,
            increase_only,
        }) => {
            let pair = find_pair(deps.storage, &vault.denoms())?;
            let belief_price = query_belief_price(&deps.querier, &pair, vault.get_swap_denom())?;
            let base_price = Decimal::from_ratio(vault.swap_amount, base_receive_amount);
            let scaled_price_delta = base_price.abs_diff(belief_price) / base_price * multiplier;

            if belief_price > base_price {
                if increase_only {
                    Decimal::one()
                } else {
                    Decimal::one()
                        .checked_sub(scaled_price_delta)
                        .unwrap_or_else(|_| Decimal::zero())
                }
            } else {
                Decimal::one()
                    .checked_add(scaled_price_delta)
                    .unwrap_or_else(|_| Decimal::one())
            }
        }
        Some(strategy) => get_swap_adjustment(deps.storage, strategy, env.block.time),
        None => Decimal::one(),
    };

    let adjusted_amount = vault.swap_amount * swap_adjustment;

    Ok(Coin::new(
        min(adjusted_amount, vault.balance.amount).into(),
        vault.get_swap_denom(),
    ))
}

pub fn get_risk_weighted_average_model_id(
    block_time: &Timestamp,
    balance: &Coin,
    swap_amount: &Uint128,
    time_interval: &TimeInterval,
) -> u8 {
    let execution_duration = get_total_execution_duration(
        *block_time,
        (balance
            .amount
            .checked_div(*swap_amount)
            .expect("deposit divided by swap amount should be larger than 0"))
        .into(),
        time_interval,
    );

    match execution_duration.num_days() {
        0..=32 => 30,
        33..=38 => 35,
        39..=44 => 40,
        45..=51 => 45,
        52..=57 => 50,
        58..=65 => 55,
        66..=77 => 60,
        78..=96 => 70,
        97..=123 => 80,
        _ => 90,
    }
}

pub fn get_performance_factor(vault: &Vault, current_price: Decimal) -> StdResult<Decimal> {
    match &vault.performance_assessment_strategy {
        Some(PerformanceAssessmentStrategy::CompareToStandardDca {
            swapped_amount,
            received_amount,
        }) => {
            let vault_total_value = vault.deposited_amount.amount - vault.swapped_amount.amount
                + vault.received_amount.amount * current_price;

            let standard_dca_vault_total_value = vault.deposited_amount.amount
                - swapped_amount.amount
                + received_amount.amount * current_price;

            Ok(Decimal::from_ratio(
                vault_total_value,
                standard_dca_vault_total_value,
            ))
        }
        None => panic!("vault {} has no performance strategy", vault.id),
    }
}

pub fn price_threshold_exceeded(
    swap_amount: Uint128,
    minimum_receive_amount: Option<Uint128>,
    belief_price: Decimal,
) -> StdResult<bool> {
    minimum_receive_amount.map_or(Ok(false), |minimum_receive_amount| {
        let swap_amount_as_decimal = Decimal::from_ratio(swap_amount, Uint128::one());

        let receive_amount_at_price = swap_amount_as_decimal
            .checked_div(belief_price)
            .expect("belief price should be larger than 0");

        let minimum_receive_amount_as_decimal =
            Decimal::from_ratio(minimum_receive_amount, Uint128::one());

        Ok(receive_amount_at_price < minimum_receive_amount_as_decimal)
    })
}

pub fn simulate_standard_dca_execution(
    mut response: Response,
    querier: &QuerierWrapper,
    storage: &mut dyn Storage,
    env: &Env,
    vault: Vault,
    belief_price: Decimal,
) -> StdResult<(Vault, Response)> {
    vault.performance_assessment_strategy.clone().map_or(
        Ok((vault.clone(), response.clone())),
        |performance_assessment_strategy| {
            let swap_amount = min(
                performance_assessment_strategy
                    .standard_dca_balance(vault.deposited_amount.clone())
                    .amount,
                vault.swap_amount,
            );

            if swap_amount.is_zero() {
                return Ok((vault, response));
            }

            let pair = find_pair(storage, &vault.denoms())?;

            let actual_price = query_price(
                querier,
                env,
                &pair,
                &Coin::new(swap_amount.into(), vault.get_swap_denom()),
            )?;

            if price_threshold_exceeded(swap_amount, vault.minimum_receive_amount, belief_price)? {
                create_event(
                    storage,
                    EventBuilder::new(
                        vault.id,
                        env.block.clone(),
                        EventData::SimulatedDcaVaultExecutionSkipped {
                            reason: ExecutionSkippedReason::PriceThresholdExceeded {
                                price: belief_price,
                            },
                        },
                    ),
                )?;

                response = response
                    .add_attribute("simulated_execution_skipped", "price_threshold_exceeded");

                return Ok((vault, response));
            }

            if let Some(slippage_tolerance) = vault.slippage_tolerance {
                let slippage = calculate_slippage(actual_price, belief_price);

                if slippage > slippage_tolerance {
                    create_event(
                        storage,
                        EventBuilder::new(
                            vault.id,
                            env.block.clone(),
                            EventData::SimulatedDcaVaultExecutionSkipped {
                                reason: ExecutionSkippedReason::SlippageToleranceExceeded,
                            },
                        ),
                    )?;

                    response = response.add_attribute(
                        "simulated_execution_skipped",
                        "slippage_tolerance_exceeded",
                    );

                    return Ok((vault, response));
                }
            }

            let fee_rate =
                get_swap_fee_rate(storage, &vault)? + get_delegation_fee_rate(storage, &vault)?;

            let received_amount_before_fee = swap_amount * (Decimal::one() / actual_price);
            let fee_amount = received_amount_before_fee * fee_rate;
            let received_amount_after_fee = received_amount_before_fee - fee_amount;

            let vault = Vault {
                performance_assessment_strategy: vault.performance_assessment_strategy.map(
                    |PerformanceAssessmentStrategy::CompareToStandardDca {
                         swapped_amount,
                         received_amount,
                     }| {
                        PerformanceAssessmentStrategy::CompareToStandardDca {
                            swapped_amount: add_to(swapped_amount, swap_amount),
                            received_amount: add_to(received_amount, received_amount_after_fee),
                        }
                    },
                ),
                ..vault
            };

            update_vault(storage, &vault)?;

            let coin_sent = Coin::new(swap_amount.into(), vault.get_swap_denom());
            let coin_received = Coin::new(
                received_amount_before_fee.into(),
                vault.target_denom.clone(),
            );
            let total_fee = Coin::new(fee_amount.into(), vault.target_denom.clone());

            create_event(
                storage,
                EventBuilder::new(
                    vault.id,
                    env.block.clone(),
                    EventData::SimulatedDcaVaultExecutionCompleted {
                        sent: coin_sent.clone(),
                        received: coin_received.clone(),
                        fee: total_fee.clone(),
                    },
                ),
            )?;

            response = response
                .add_attribute("simulated_swapped_amount", coin_sent.to_string())
                .add_attribute("simulated_received_amount", coin_received.to_string())
                .add_attribute("simulated_total_fee", total_fee.to_string());

            Ok((vault, response))
        },
    )
}

#[cfg(test)]
mod get_swap_amount_tests {
    use std::str::FromStr;

    use super::*;
    use crate::{
        constants::{ONE, SWAP_FEE_RATE, TWO_MICRONS},
        state::swap_adjustments::update_swap_adjustment,
        tests::{
            helpers::{instantiate_contract, setup_vault},
            mocks::{calc_mock_dependencies, ADMIN, DENOM_UOSMO},
        },
        types::swap_adjustment_strategy::SwapAdjustmentStrategy,
    };
    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary, StdError,
    };
    use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceResponse;

    #[test]
    fn should_return_full_balance_when_vault_has_low_funds() {
        let deps = mock_dependencies();
        let env = mock_env();

        let vault = Vault {
            balance: Coin::new(ONE.into(), DENOM_UOSMO),
            swap_amount: ONE + ONE,
            ..Vault::default()
        };

        assert_eq!(
            get_swap_amount(&deps.as_ref(), &env, &vault).unwrap(),
            vault.balance
        );
    }

    #[test]
    fn should_return_swap_amount_when_vault_has_enough_funds() {
        let deps = mock_dependencies();
        let env = mock_env();

        let vault = Vault::default();

        assert_eq!(
            get_swap_amount(&deps.as_ref(), &env, &vault)
                .unwrap()
                .amount,
            vault.swap_amount
        );
    }

    #[test]
    fn rwa_should_return_adjusted_swap_amount() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let swap_adjustment = Decimal::percent(90);

        update_swap_adjustment(
            deps.as_mut().storage,
            vault.swap_adjustment_strategy.clone().unwrap(),
            swap_adjustment,
            env.block.time,
        )
        .unwrap();

        assert_eq!(
            get_swap_amount(&deps.as_ref(), &env, &vault)
                .unwrap()
                .amount,
            vault.swap_amount * swap_adjustment
        );
    }

    #[test]
    fn rwa_should_return_adjusted_swap_amount_with_low_funds_and_reduced_swap_amount() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: coin((ONE / TWO_MICRONS).into(), DENOM_UOSMO),
                swap_amount: ONE,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let swap_adjustment = Decimal::percent(17);

        update_swap_adjustment(
            deps.as_mut().storage,
            vault.swap_adjustment_strategy.clone().unwrap(),
            swap_adjustment,
            env.block.time,
        )
        .unwrap();

        assert_eq!(
            get_swap_amount(&deps.as_ref(), &env, &vault)
                .unwrap()
                .amount,
            vault.swap_amount * swap_adjustment
        );
    }

    #[test]
    fn rwa_should_return_vault_balance_with_low_funds_and_increased_swap_amount() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: coin((ONE / TWO_MICRONS).into(), DENOM_UOSMO),
                swap_amount: ONE,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let swap_adjustment = Decimal::percent(120);

        update_swap_adjustment(
            deps.as_mut().storage,
            vault.swap_adjustment_strategy.clone().unwrap(),
            swap_adjustment,
            env.block.time,
        )
        .unwrap();

        assert_eq!(
            get_swap_amount(&deps.as_ref(), &env, &vault)
                .unwrap()
                .amount,
            vault.balance.amount
        );
    }

    #[test]
    fn raw_should_return_vault_balance_with_increased_swap_amount_above_balance() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: coin((ONE * Decimal::percent(150)).into(), DENOM_UOSMO),
                swap_amount: ONE,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let swap_adjustment = Decimal::percent(200);

        update_swap_adjustment(
            deps.as_mut().storage,
            vault.swap_adjustment_strategy.clone().unwrap(),
            swap_adjustment,
            env.block.time,
        )
        .unwrap();

        assert_eq!(
            get_swap_amount(&deps.as_ref(), &env, &vault)
                .unwrap()
                .amount,
            vault.balance.amount
        );
    }

    #[test]
    fn ws_should_return_decreased_swap_amount_when_price_increased() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let multiplier = Decimal::percent(300);
        let base_receive_amount = ONE;

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount,
                    multiplier,
                    increase_only: false,
                }),
                ..Vault::default()
            },
        );

        let base_price = Decimal::from_ratio(vault.swap_amount, base_receive_amount);
        let current_price =
            Decimal::percent(120) * (Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap());

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: "1.2".to_string(),
            }),
            _ => Err(StdError::generic_err("message not customised")),
        });

        let swap_amount = get_swap_amount(&deps.as_ref(), &env, &vault).unwrap();

        assert_eq!(
            swap_amount.amount,
            vault.swap_amount
                * (Decimal::one() - (current_price.abs_diff(base_price) / base_price) * multiplier)
        );
    }

    #[test]
    fn ws_should_not_return_decreased_swap_amount_when_price_increased_but_increase_only_is_true() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let multiplier = Decimal::percent(300);
        let base_receive_amount = ONE;

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount,
                    multiplier,
                    increase_only: true,
                }),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: "1.2".to_string(),
            }),
            _ => Err(StdError::generic_err("message not customised")),
        });

        let swap_amount = get_swap_amount(&deps.as_ref(), &env, &vault).unwrap();

        assert_eq!(swap_amount.amount, vault.swap_amount);
    }

    #[test]
    fn ws_should_return_increased_swap_amount_when_price_decreased() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let base_receive_amount = ONE;
        let multiplier = Decimal::percent(150);

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount,
                    multiplier,
                    increase_only: false,
                }),
                ..Vault::default()
            },
        );

        let base_price = Decimal::from_ratio(vault.swap_amount, base_receive_amount);
        let current_price =
            Decimal::percent(70) * (Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap());

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: "0.7".to_string(),
            }),
            _ => Err(StdError::generic_err("message not customised")),
        });

        let swap_amount = get_swap_amount(&deps.as_ref(), &env, &vault).unwrap();

        assert_eq!(
            swap_amount.amount,
            vault.swap_amount
                * (Decimal::one() + (current_price.abs_diff(base_price) / base_price) * multiplier)
        );
    }

    #[test]
    fn ws_should_return_swap_amount_zero_when_price_increased_enough() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let base_receive_amount = ONE;
        let multiplier = Decimal::percent(300);

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount,
                    multiplier,
                    increase_only: false,
                }),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: "2.0".to_string(),
            }),
            _ => Err(StdError::generic_err("message not customised")),
        });

        let swap_amount = get_swap_amount(&deps.as_ref(), &env, &vault).unwrap();

        assert_eq!(swap_amount.amount, Uint128::zero());
    }
}

#[cfg(test)]
mod price_threshold_exceeded_tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use std::str::FromStr;

    #[test]
    fn should_not_be_exceeded_when_price_is_below_threshold() {
        let deps = mock_dependencies();
        let env = mock_env();

        let vault = Vault {
            swap_amount: Uint128::new(100),
            minimum_receive_amount: Some(Uint128::new(50)),
            ..Vault::default()
        };

        assert_eq!(
            price_threshold_exceeded(
                get_swap_amount(&deps.as_ref(), &env, &vault)
                    .unwrap()
                    .amount,
                vault.minimum_receive_amount,
                Decimal::from_str("1.9").unwrap()
            ),
            Ok(false)
        );
    }

    #[test]
    fn should_not_be_exceeded_when_price_equals_threshold() {
        let deps = mock_dependencies();
        let env = mock_env();

        let vault = Vault {
            swap_amount: Uint128::new(100),
            minimum_receive_amount: Some(Uint128::new(50)),
            ..Vault::default()
        };

        assert_eq!(
            price_threshold_exceeded(
                get_swap_amount(&deps.as_ref(), &env, &vault)
                    .unwrap()
                    .amount,
                vault.minimum_receive_amount,
                Decimal::from_str("2.0").unwrap()
            ),
            Ok(false)
        );
    }

    #[test]
    fn should_be_exceeded_when_price_is_above_threshold() {
        let deps = mock_dependencies();
        let env = mock_env();

        let vault = Vault {
            swap_amount: Uint128::new(100),
            minimum_receive_amount: Some(Uint128::new(50)),
            ..Vault::default()
        };

        assert_eq!(
            price_threshold_exceeded(
                get_swap_amount(&deps.as_ref(), &env, &vault)
                    .unwrap()
                    .amount,
                vault.minimum_receive_amount,
                Decimal::from_str("2.1").unwrap()
            ),
            Ok(true)
        );
    }
}

#[cfg(test)]
mod get_risk_weighted_average_strategy_model_id_tests {
    use crate::{
        constants::{ONE, TEN},
        helpers::vault::get_risk_weighted_average_model_id,
        types::time_interval::TimeInterval,
    };
    use cosmwasm_std::{testing::mock_env, Coin, Uint128};

    #[test]
    fn should_return_30_when_days_less_than_30() {
        let env = mock_env();

        let balance = Coin::new(TEN.into(), "base");
        let swap_amount = ONE;

        assert_eq!(
            get_risk_weighted_average_model_id(
                &env.block.time,
                &balance,
                &swap_amount,
                &TimeInterval::Daily
            ),
            30
        );
    }

    #[test]
    fn should_return_90_when_days_more_than_123() {
        let env = mock_env();

        let balance = Coin::new((ONE * Uint128::new(124)).into(), "base");
        let swap_amount = ONE;

        assert_eq!(
            get_risk_weighted_average_model_id(
                &env.block.time,
                &balance,
                &swap_amount,
                &TimeInterval::Daily
            ),
            90
        );
    }

    #[test]
    fn should_return_60_when_days_equals_70() {
        let env = mock_env();

        let balance = Coin::new((ONE * Uint128::new(70)).into(), "base");
        let swap_amount = ONE;

        assert_eq!(
            get_risk_weighted_average_model_id(
                &env.block.time,
                &balance,
                &swap_amount,
                &TimeInterval::Daily
            ),
            60
        );
    }
}

#[cfg(test)]
mod get_performance_factor_tests {
    use crate::{
        helpers::vault::get_performance_factor,
        types::{performance_assessment_strategy::PerformanceAssessmentStrategy, vault::Vault},
    };
    use cosmwasm_std::{Coin, Decimal, Uint128};
    use std::str::FromStr;

    fn get_vault(
        total_deposit: Uint128,
        swapped_amount: Uint128,
        standard_dca_swapped_amount: Uint128,
        received_amount: Uint128,
        standard_dca_received_amount: Uint128,
    ) -> Vault {
        Vault {
            swap_amount: swapped_amount / Uint128::new(2),
            escrow_level: Decimal::percent(5),
            balance: Coin {
                denom: "swap_denom".to_string(),
                amount: total_deposit - swapped_amount,
            },
            deposited_amount: Coin {
                denom: "swap_denom".to_string(),
                amount: total_deposit,
            },
            swapped_amount: Coin {
                denom: "swap_denom".to_string(),
                amount: swapped_amount,
            },
            received_amount: Coin {
                denom: "receive_denom".to_string(),
                amount: received_amount,
            },
            performance_assessment_strategy: Some(
                PerformanceAssessmentStrategy::CompareToStandardDca {
                    swapped_amount: Coin::new(
                        standard_dca_swapped_amount.into(),
                        "swap_denom".to_string(),
                    ),
                    received_amount: Coin::new(
                        standard_dca_received_amount.into(),
                        "receive_denom".to_string(),
                    ),
                },
            ),
            ..Vault::default()
        }
    }

    fn assert_peformance_factor(
        total_deposit: Uint128,
        swapped_amount: Uint128,
        standard_dca_swapped_amount: Uint128,
        received_amount: Uint128,
        standard_dca_received_amount: Uint128,
        current_price: Decimal,
        expected_performance_factor: Decimal,
    ) {
        let vault = get_vault(
            total_deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
        );

        let factor = get_performance_factor(&vault, current_price).unwrap();
        assert_eq!(factor, expected_performance_factor);
    }

    #[test]
    fn performance_is_equal_when_same_amount_swapped_and_received() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(900);
        let received_amount = Uint128::new(1100);
        let standard_dca_swapped_amount = Uint128::new(900);
        let standard_dca_received_amount = Uint128::new(1100);
        let current_price = Decimal::from_str("1.1").unwrap();
        let expected_performance_factor = Decimal::percent(100);

        assert_peformance_factor(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_performance_factor,
        );
    }

    #[test]
    fn performance_is_better_when_same_amount_swapped_and_more_received() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(900);
        let received_amount = Uint128::new(1100);
        let standard_dca_swapped_amount = Uint128::new(900);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1.1").unwrap();
        let expected_performance_factor = Decimal::percent(105);

        assert_peformance_factor(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_performance_factor,
        );
    }

    #[test]
    fn performance_is_better_when_less_swapped_and_same_amount_received() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(800);
        let received_amount = Uint128::new(1200);
        let standard_dca_swapped_amount = Uint128::new(900);
        let standard_dca_received_amount = Uint128::new(1200);
        let current_price = Decimal::from_str("1.1").unwrap();
        let expected_performance_factor = Decimal::from_str("1.041322314049586776").unwrap();

        assert_peformance_factor(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_performance_factor,
        );
    }

    #[test]
    fn performance_is_worse_when_more_swapped_and_same_amount_received() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1200);
        let standard_dca_swapped_amount = Uint128::new(900);
        let standard_dca_received_amount = Uint128::new(1200);
        let current_price = Decimal::from_str("1.1").unwrap();
        let expected_performance_factor = Decimal::from_str("0.958677685950413223").unwrap();

        assert_peformance_factor(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_performance_factor,
        );
    }

    #[test]
    fn performance_is_worse_when_same_amount_swapped_and_less_received() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1100);
        let current_price = Decimal::from_str("1.1").unwrap();
        let expected_performance_factor = Decimal::from_str("0.950226244343891402").unwrap();

        assert_peformance_factor(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_performance_factor,
        );
    }
}

#[cfg(test)]
mod simulate_standard_dca_execution_tests {
    use super::simulate_standard_dca_execution;
    use crate::tests::helpers::setup_vault;
    use crate::tests::mocks::DENOM_STAKE;
    use crate::types::event::{Event, EventData, ExecutionSkippedReason};
    use crate::types::performance_assessment_strategy::PerformanceAssessmentStrategy;
    use crate::{
        constants::{ONE, TEN},
        handlers::get_events_by_resource_id::get_events_by_resource_id_handler,
        helpers::fees::{get_delegation_fee_rate, get_swap_fee_rate},
        tests::{
            helpers::instantiate_contract,
            mocks::{calc_mock_dependencies, ADMIN, DENOM_UOSMO},
        },
        types::{swap_adjustment_strategy::SwapAdjustmentStrategy, vault::Vault},
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Decimal,
    };
    use cosmwasm_std::{Coin, Response};

    #[test]
    fn for_standard_dca_vault_succeeds() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let vault = Vault::default();

        let (updated_vault, _) = simulate_standard_dca_execution(
            Response::new(),
            &deps.as_ref().querier,
            mock_dependencies().as_mut().storage,
            &env,
            vault.clone(),
            Decimal::one(),
        )
        .unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert_eq!(events.len(), 0);
        assert_eq!(updated_vault, vault);
    }

    #[test]
    fn with_finished_standard_dca_succeeds() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let vault = Vault {
            performance_assessment_strategy: Some(
                PerformanceAssessmentStrategy::CompareToStandardDca {
                    swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                    received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                },
            ),
            ..Vault::default()
        };

        let (updated_vault, _) = simulate_standard_dca_execution(
            Response::new(),
            &deps.as_ref().querier,
            mock_dependencies().as_mut().storage,
            &env,
            vault.clone(),
            Decimal::one(),
        )
        .unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert_eq!(events.len(), 0);
        assert_eq!(updated_vault, vault);
    }

    #[test]
    fn publishes_simulated_execution_skipped_event_when_price_threshold_exceeded() {
        let deps = calc_mock_dependencies();
        let mut storage_deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(storage_deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let vault = setup_vault(
            storage_deps.as_mut(),
            env.clone(),
            Vault {
                swap_amount: ONE,
                minimum_receive_amount: Some(ONE + ONE),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                escrow_level: Decimal::percent(5),
                ..Vault::default()
            },
        );

        let belief_price = Decimal::one();

        simulate_standard_dca_execution(
            Response::new(),
            &deps.as_ref().querier,
            storage_deps.as_mut().storage,
            &env,
            vault.clone(),
            belief_price,
        )
        .unwrap();

        let events =
            get_events_by_resource_id_handler(storage_deps.as_ref(), vault.id, None, None, None)
                .unwrap()
                .events;

        assert!(events.contains(&Event {
            id: 1,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::SimulatedDcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::PriceThresholdExceeded {
                    price: belief_price
                }
            }
        }))
    }

    #[test]
    fn publishes_simulated_execution_skipped_event_when_slippage_exceeded() {
        let deps = calc_mock_dependencies();
        let mut storage_deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(storage_deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let vault = setup_vault(
            storage_deps.as_mut(),
            env.clone(),
            Vault {
                swap_amount: TEN,
                slippage_tolerance: Some(Decimal::percent(2)),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                escrow_level: Decimal::percent(5),
                ..Vault::default()
            },
        );

        simulate_standard_dca_execution(
            Response::new(),
            &deps.as_ref().querier,
            storage_deps.as_mut().storage,
            &env,
            vault.clone(),
            Decimal::one(),
        )
        .unwrap();

        let events =
            get_events_by_resource_id_handler(storage_deps.as_ref(), vault.id, None, None, None)
                .unwrap()
                .events;

        assert!(events.contains(&Event {
            id: 1,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::SimulatedDcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::SlippageToleranceExceeded
            }
        }))
    }

    #[test]
    fn publishes_simulated_execution_completed_event() {
        let deps = calc_mock_dependencies();
        let mut storage_deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(storage_deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let vault = setup_vault(
            storage_deps.as_mut(),
            env.clone(),
            Vault {
                swap_amount: ONE,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                escrow_level: Decimal::percent(5),
                ..Vault::default()
            },
        );

        let belief_price = Decimal::one();

        simulate_standard_dca_execution(
            Response::new(),
            &deps.as_ref().querier,
            storage_deps.as_mut().storage,
            &env,
            vault.clone(),
            belief_price,
        )
        .unwrap();

        let events =
            get_events_by_resource_id_handler(storage_deps.as_ref(), vault.id, None, None, None)
                .unwrap()
                .events;

        let fee_rate = get_swap_fee_rate(storage_deps.as_ref().storage, &vault).unwrap()
            + get_delegation_fee_rate(storage_deps.as_ref().storage, &vault).unwrap();

        let received_amount = vault.swap_amount * Decimal::one();
        let fee_amount = received_amount * fee_rate;

        assert!(events.contains(&Event {
            id: 1,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::SimulatedDcaVaultExecutionCompleted {
                sent: Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
                received: Coin::new(received_amount.into(), vault.target_denom.clone()),
                fee: Coin::new(fee_amount.into(), vault.target_denom.clone())
            }
        }));
    }

    #[test]
    fn updates_the_standard_dca_statistics() {
        let deps = calc_mock_dependencies();
        let mut storage_deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(storage_deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let belief_price = Decimal::one();

        let vault = setup_vault(
            storage_deps.as_mut(),
            env.clone(),
            Vault {
                swap_amount: ONE,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                escrow_level: Decimal::percent(5),
                ..Vault::default()
            },
        );

        let (vault, _) = simulate_standard_dca_execution(
            Response::new(),
            &deps.as_ref().querier,
            storage_deps.as_mut().storage,
            &env,
            vault,
            belief_price,
        )
        .unwrap();

        let fee_rate = get_swap_fee_rate(storage_deps.as_ref().storage, &vault).unwrap()
            + get_delegation_fee_rate(storage_deps.as_ref().storage, &vault).unwrap();

        let received_amount_before_fee = vault.swap_amount * Decimal::one();
        let fee_amount = received_amount_before_fee * fee_rate;
        let received_amount_after_fee = received_amount_before_fee - fee_amount;

        let performance_assessment_strategy =
            vault.performance_assessment_strategy.clone().unwrap();

        assert_eq!(
            performance_assessment_strategy.standard_dca_swapped_amount(),
            Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
        );
        assert_eq!(
            performance_assessment_strategy.standard_dca_received_amount(),
            Coin::new(received_amount_after_fee.into(), vault.target_denom)
        );
    }
}
