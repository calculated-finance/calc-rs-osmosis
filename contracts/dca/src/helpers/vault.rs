use super::{
    coin::add_to,
    fees::{get_delegation_fee_rate, get_swap_fee_rate},
    price::{calculate_slippage, query_price},
    time::get_total_execution_duration,
};
use crate::{
    handlers::get_swap_adjustment::get_swap_adjustment_handler,
    state::{events::create_event, pairs::find_pair, vaults::update_vault},
    types::{
        event::{EventBuilder, EventData, ExecutionSkippedReason},
        position_type::PositionType,
        swap_adjustment_strategy::SwapAdjustmentStrategy,
        time_interval::TimeInterval,
        vault::Vault,
    },
};
use cosmwasm_std::{
    Coin, Decimal, Deps, Env, QuerierWrapper, StdResult, Storage, Timestamp, Uint128,
};
use std::cmp::min;

pub fn get_position_type(deps: &Deps, vault: &Vault) -> StdResult<PositionType> {
    let pair = find_pair(deps.storage, &vault.denoms())?;
    Ok(pair.position_type(vault.get_swap_denom()))
}

pub fn get_swap_amount(deps: &Deps, env: &Env, vault: &Vault) -> StdResult<Coin> {
    let swap_adjustment =
        get_swap_adjustment_handler(deps.storage, env, vault.swap_adjustment_strategy.clone())
            .unwrap_or(Decimal::one());

    let adjusted_amount = vault.swap_amount * swap_adjustment;

    Ok(Coin {
        denom: vault.get_swap_denom(),
        amount: min(adjusted_amount, vault.balance.amount),
    })
}

pub fn get_dca_plus_model_id(
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

pub fn get_dca_plus_performance_factor(
    vault: &Vault,
    current_price: Decimal,
) -> StdResult<Decimal> {
    let swap_adjustment_strategy = vault
        .swap_adjustment_strategy
        .clone()
        .expect("Only DCA plus vaults should try to get performance");

    let dca_plus_total_value = swap_adjustment_strategy.dca_plus_total_deposit().amount
        - vault.swapped_amount.amount
        + vault.received_amount.amount * current_price;

    let standard_dca_total_value = swap_adjustment_strategy.dca_plus_total_deposit().amount
        - swap_adjustment_strategy
            .dca_plus_standard_dca_swapped_amount()
            .amount
        + swap_adjustment_strategy
            .dca_plus_standard_dca_received_amount()
            .amount
            * current_price;

    Ok(Decimal::from_ratio(
        dca_plus_total_value,
        standard_dca_total_value,
    ))
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
    querier: &QuerierWrapper,
    storage: &mut dyn Storage,
    env: &Env,
    vault: Vault,
    belief_price: Decimal,
) -> StdResult<Vault> {
    vault
        .swap_adjustment_strategy
        .clone()
        .map_or(Ok(vault.clone()), |swap_adjustment_strategy| {
            let swap_amount = min(
                swap_adjustment_strategy
                    .dca_plus_standard_dca_balance()
                    .amount,
                vault.swap_amount,
            );

            if swap_amount.is_zero() {
                return Ok(vault);
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

                return Ok(vault);
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

                    return Ok(vault);
                }
            }

            let fee_rate =
                get_swap_fee_rate(storage, &vault)? + get_delegation_fee_rate(storage, &vault)?;

            let received_amount_before_fee = swap_amount * (Decimal::one() / actual_price);
            let fee_amount = received_amount_before_fee * fee_rate;
            let received_amount_after_fee = received_amount_before_fee - fee_amount;

            let vault = Vault {
                swap_adjustment_strategy: vault.swap_adjustment_strategy.map(
                    |SwapAdjustmentStrategy::DcaPlus {
                         escrow_level,
                         model_id,
                         total_deposit,
                         standard_dca_swapped_amount,
                         standard_dca_received_amount,
                         escrowed_balance,
                     }| SwapAdjustmentStrategy::DcaPlus {
                        standard_dca_swapped_amount: add_to(
                            standard_dca_swapped_amount,
                            swap_amount,
                        ),
                        standard_dca_received_amount: add_to(
                            standard_dca_received_amount,
                            received_amount_after_fee,
                        ),
                        escrow_level,
                        model_id,
                        escrowed_balance,
                        total_deposit,
                    },
                ),
                ..vault
            };

            update_vault(storage, &vault)?;

            create_event(
                storage,
                EventBuilder::new(
                    vault.id,
                    env.block.clone(),
                    EventData::SimulatedDcaVaultExecutionCompleted {
                        sent: Coin::new(swap_amount.into(), vault.get_swap_denom()),
                        received: Coin::new(
                            received_amount_before_fee.into(),
                            vault.target_denom.clone(),
                        ),
                        fee: Coin::new(fee_amount.into(), vault.target_denom.clone()),
                    },
                ),
            )?;

            Ok(vault)
        })
}

#[cfg(test)]
mod get_swap_amount_tests {
    use super::*;
    use crate::{
        constants::{ONE, TWO_MICRONS},
        state::swap_adjustments::update_swap_adjustments,
        tests::{helpers::setup_vault, mocks::DENOM_UOSMO},
        types::{position_type::PositionType, swap_adjustment_strategy::SwapAdjustmentStrategy},
    };
    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies, mock_env},
    };

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
    fn should_return_adjusted_swap_amount_for_dca_plus_vault() {
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

        update_swap_adjustments(
            deps.as_mut().storage,
            PositionType::Enter,
            vec![(30, swap_adjustment)],
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
    fn should_return_adjusted_swap_amount_for_dca_plus_vault_with_low_funds_and_reduced_swap_amount(
    ) {
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

        update_swap_adjustments(
            deps.as_mut().storage,
            PositionType::Enter,
            vec![(30, swap_adjustment)],
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
    fn should_return_vault_balance_for_dca_plus_vault_with_low_funds_and_increased_swap_amount() {
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

        update_swap_adjustments(
            deps.as_mut().storage,
            PositionType::Exit,
            vec![(30, swap_adjustment)],
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
    fn should_return_vault_balance_for_dca_plus_vault_with_increased_swap_amount_above_balance() {
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

        update_swap_adjustments(
            deps.as_mut().storage,
            PositionType::Enter,
            vec![(30, swap_adjustment)],
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
mod get_dca_plus_model_id_tests {
    use crate::{
        constants::{ONE, TEN},
        helpers::vault::get_dca_plus_model_id,
        types::time_interval::TimeInterval,
    };
    use cosmwasm_std::{testing::mock_env, Coin, Uint128};

    #[test]
    fn should_return_30_when_days_less_than_30() {
        let env = mock_env();

        let balance = Coin::new(TEN.into(), "base");
        let swap_amount = ONE;

        assert_eq!(
            get_dca_plus_model_id(
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
            get_dca_plus_model_id(
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
            get_dca_plus_model_id(
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
mod get_dca_plus_performance_factor_tests {
    use crate::{
        helpers::vault::get_dca_plus_performance_factor,
        types::{swap_adjustment_strategy::SwapAdjustmentStrategy, vault::Vault},
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
        let escrow_level = Decimal::percent(5);

        Vault {
            swap_amount: swapped_amount / Uint128::new(2),
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
            swap_adjustment_strategy: Some(SwapAdjustmentStrategy::DcaPlus {
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
                escrow_level: Decimal::percent(5),
            }),
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

        let factor = get_dca_plus_performance_factor(&vault, current_price).unwrap();
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
    use cosmwasm_std::Coin;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Decimal,
    };

    #[test]
    fn for_non_dca_plus_vault_succeeds() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let vault = Vault::default();

        let updated_vault = simulate_standard_dca_execution(
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
            swap_adjustment_strategy: Some(SwapAdjustmentStrategy::DcaPlus {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                escrowed_balance: Coin::new((TEN * Decimal::percent(5)).into(), DENOM_STAKE),
                model_id: 30,
                escrow_level: Decimal::percent(5),
            }),
            ..Vault::default()
        };

        let updated_vault = simulate_standard_dca_execution(
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
                ..Vault::default()
            },
        );

        let belief_price = Decimal::one();

        simulate_standard_dca_execution(
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
                ..Vault::default()
            },
        );

        simulate_standard_dca_execution(
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
                ..Vault::default()
            },
        );

        let belief_price = Decimal::one();

        simulate_standard_dca_execution(
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
                ..Vault::default()
            },
        );

        let vault = simulate_standard_dca_execution(
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

        let swap_adjustment_strategy = vault.swap_adjustment_strategy.clone().unwrap();

        assert_eq!(
            swap_adjustment_strategy.dca_plus_standard_dca_swapped_amount(),
            Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
        );
        assert_eq!(
            swap_adjustment_strategy.dca_plus_standard_dca_received_amount(),
            Coin::new(received_amount_after_fee.into(), vault.target_denom)
        );
    }
}