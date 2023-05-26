use crate::{
    error::ContractError,
    helpers::validation::{
        assert_destination_allocations_add_up_to_one,
        assert_destination_callback_addresses_are_valid, assert_destinations_limit_is_not_breached,
        assert_label_is_no_longer_than_100_characters, assert_no_destination_allocations_are_zero,
        assert_slippage_tolerance_is_less_than_or_equal_to_one, assert_time_interval_is_valid,
        assert_vault_is_not_cancelled, assert_weighted_scale_multiplier_is_no_more_than_10,
        asset_sender_is_vault_owner,
    },
    state::vaults::{get_vault, update_vault},
    types::{
        destination::Destination,
        swap_adjustment_strategy::{SwapAdjustmentStrategy, SwapAdjustmentStrategyParams},
        time_interval::TimeInterval,
    },
};
use cosmwasm_std::{Decimal, DepsMut, MessageInfo, Response, Uint128};

pub fn update_vault_handler(
    deps: DepsMut,
    info: MessageInfo,
    vault_id: Uint128,
    label: Option<String>,
    destinations: Option<Vec<Destination>>,
    slippage_tolerance: Option<Decimal>,
    minimum_receive_amount: Option<Uint128>,
    time_interval: Option<TimeInterval>,
    swap_adjustment_strategy: Option<SwapAdjustmentStrategyParams>,
) -> Result<Response, ContractError> {
    let mut vault = get_vault(deps.storage, vault_id)?;

    asset_sender_is_vault_owner(vault.owner.clone(), info.sender)?;
    assert_vault_is_not_cancelled(&vault)?;

    let mut response = Response::default()
        .add_attribute("update_vault", "true")
        .add_attribute("vault_id", vault.id)
        .add_attribute("owner", vault.owner.clone());

    if let Some(label) = label {
        assert_label_is_no_longer_than_100_characters(&label)?;

        vault.label = Some(label.clone());
        response = response.add_attribute("label", label);
    }

    if let Some(mut destinations) = destinations {
        if destinations.is_empty() {
            destinations.push(Destination {
                allocation: Decimal::percent(100),
                address: vault.owner.clone(),
                msg: None,
            });
        }

        assert_destinations_limit_is_not_breached(&destinations)?;
        assert_destination_callback_addresses_are_valid(deps.as_ref(), &destinations)?;
        assert_no_destination_allocations_are_zero(&destinations)?;
        assert_destination_allocations_add_up_to_one(&destinations)?;

        vault.destinations = destinations.clone();
        response = response.add_attribute("destinations", format!("{:?}", destinations));
    }

    if let Some(slippage_tolerance) = slippage_tolerance {
        assert_slippage_tolerance_is_less_than_or_equal_to_one(slippage_tolerance)?;
        vault.slippage_tolerance = slippage_tolerance;
        response = response.add_attribute("slippage_tolerance", slippage_tolerance.to_string());
    }

    if let Some(minimum_receive_amount) = minimum_receive_amount {
        vault.minimum_receive_amount = Some(minimum_receive_amount);
        response = response.add_attribute("minimum_receive_amount", minimum_receive_amount);
    }

    if let Some(time_interval) = time_interval {
        assert_time_interval_is_valid(&time_interval)?;
        vault.time_interval = time_interval.clone();
        response = response.add_attribute("time_interval", time_interval);
    }

    match swap_adjustment_strategy {
        Some(SwapAdjustmentStrategyParams::WeightedScale {
            base_receive_amount,
            multiplier,
            increase_only,
        }) => match vault.swap_adjustment_strategy {
            Some(SwapAdjustmentStrategy::WeightedScale { .. }) => {
                assert_weighted_scale_multiplier_is_no_more_than_10(multiplier)?;
                vault.swap_adjustment_strategy = Some(SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount,
                    multiplier,
                    increase_only,
                })
            }
            _ => {
                return Err(ContractError::CustomError {
                    val: format!(
                        "cannot update swap adjustment strategy from {:?} to {:?}",
                        vault.swap_adjustment_strategy, swap_adjustment_strategy
                    ),
                })
            }
        },
        Some(swap_adjustment_strategy) => {
            return Err(ContractError::CustomError {
                val: format!(
                    "cannot update swap adjustment strategy from {:?} to {:?}",
                    vault.swap_adjustment_strategy, swap_adjustment_strategy
                ),
            })
        }
        _ => {}
    }

    update_vault(deps.storage, vault)?;
    Ok(response)
}

#[cfg(test)]
mod update_vault_tests {
    use super::update_vault_handler;
    use crate::{
        state::vaults::get_vault,
        tests::{
            helpers::{instantiate_contract, setup_vault},
            mocks::{ADMIN, USER},
        },
        types::{
            destination::Destination,
            position_type::PositionType,
            swap_adjustment_strategy::{
                BaseDenom, SwapAdjustmentStrategy, SwapAdjustmentStrategyParams,
            },
            time_interval::TimeInterval,
            vault::{Vault, VaultStatus},
        },
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Decimal, Uint128,
    };

    #[test]
    fn with_slippage_tolerance_larger_than_one_fails() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            Some(Decimal::percent(101)),
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: slippage tolerance must be less than or equal to 1"
        );
    }

    #[test]
    fn with_custom_time_interval_less_than_60_seconds_fails() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            None,
            None,
            Some(TimeInterval::Custom { seconds: 12 }),
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: custom time interval must be at least 60 seconds"
        );
    }

    #[test]
    fn with_label_longer_than_100_characters_fails() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let label = Some("12345678910".repeat(10).to_string());

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            label.clone(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: Vault label cannot be longer than 100 characters"
        );
    }

    #[test]
    fn for_vault_with_different_owner_fails() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let vault = setup_vault(
            deps.as_mut(),
            mock_env(),
            Vault {
                owner: Addr::unchecked("random"),
                ..Vault::default()
            },
        );

        let label = Some("My new vault".to_string());

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            label.clone(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Unauthorized");
    }

    #[test]
    fn for_cancelled_vault_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(
            deps.as_mut(),
            mock_env(),
            Vault {
                status: VaultStatus::Cancelled,
                ..Vault::default()
            },
        );

        let label = Some("My new vault".to_string());

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            label.clone(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Error: vault is already cancelled");
    }

    #[test]
    fn with_more_than_10_destinations_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(10),
                msg: None,
            };
            11
        ];

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations),
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: no more than 10 destinations can be provided"
        );
    }

    #[test]
    fn with_destination_allocations_less_than_100_percent_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(10),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(10),
                msg: None,
            },
        ];

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations),
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: destination allocations must add up to 1"
        );
    }

    #[test]
    fn with_destination_allocations_more_than_100_percent_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(50),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(51),
                msg: None,
            },
        ];

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations),
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: destination allocations must add up to 1"
        );
    }

    #[test]
    fn with_destination_with_zero_allocation_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(100),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::zero(),
                msg: None,
            },
        ];

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations),
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: all destination allocations must be greater than 0"
        );
    }

    #[test]
    fn updating_risk_weighted_average_strategy_fails() {
        let mut deps = mock_dependencies();

        let existing_swap_adjustment_strategy = Some(SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id: 30,
            base_denom: BaseDenom::Bitcoin,
            position_type: PositionType::Enter,
        });

        let vault = setup_vault(
            deps.as_mut(),
            mock_env(),
            Vault {
                swap_adjustment_strategy: existing_swap_adjustment_strategy.clone(),
                ..Vault::default()
            },
        );

        let new_swap_adjustment_strategy = SwapAdjustmentStrategyParams::RiskWeightedAverage {
            base_denom: BaseDenom::Bitcoin,
        };

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            None,
            None,
            None,
            Some(new_swap_adjustment_strategy.clone()),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Error: cannot update swap adjustment strategy from {:?} to {:?}",
                existing_swap_adjustment_strategy, new_swap_adjustment_strategy
            )
        );
    }

    #[test]
    fn changing_risk_weighted_average_strategy_fails() {
        let mut deps = mock_dependencies();

        let existing_swap_adjustment_strategy = Some(SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id: 30,
            base_denom: BaseDenom::Bitcoin,
            position_type: PositionType::Enter,
        });

        let vault = setup_vault(
            deps.as_mut(),
            mock_env(),
            Vault {
                swap_adjustment_strategy: existing_swap_adjustment_strategy.clone(),
                ..Vault::default()
            },
        );

        let new_swap_adjustment_strategy = Some(SwapAdjustmentStrategyParams::WeightedScale {
            base_receive_amount: Uint128::new(18277),
            multiplier: Decimal::percent(213),
            increase_only: false,
        });

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            None,
            None,
            None,
            new_swap_adjustment_strategy.clone(),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Error: cannot update swap adjustment strategy from {:?} to {:?}",
                existing_swap_adjustment_strategy, new_swap_adjustment_strategy
            )
        );
    }

    #[test]
    fn adding_weighted_scale_swap_adjustment_strategy_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let strategy = Some(SwapAdjustmentStrategyParams::WeightedScale {
            base_receive_amount: Uint128::new(2732),
            multiplier: Decimal::percent(150),
            increase_only: false,
        });

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            None,
            None,
            None,
            strategy.clone(),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Error: cannot update swap adjustment strategy from {:?} to {:?}",
                vault.swap_adjustment_strategy, strategy
            )
        );
    }

    #[test]
    fn updates_weighted_scale_swap_adjustment_strategy() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(
            deps.as_mut(),
            mock_env(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount: Uint128::new(2732),
                    multiplier: Decimal::percent(150),
                    increase_only: false,
                }),
                ..Vault::default()
            },
        );

        let base_receive_amount = Uint128::new(212831);
        let multiplier = Decimal::percent(300);
        let increase_only = true;

        let strategy = Some(SwapAdjustmentStrategyParams::WeightedScale {
            base_receive_amount,
            multiplier,
            increase_only,
        });

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            None,
            None,
            None,
            strategy,
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(
            updated_vault.swap_adjustment_strategy,
            Some(SwapAdjustmentStrategy::WeightedScale {
                base_receive_amount,
                multiplier,
                increase_only,
            })
        );
    }

    #[test]
    fn updates_the_vault_label() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let label = Some("123456789".repeat(10).to_string());

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            label.clone(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(updated_vault.label, label);
    }

    #[test]
    fn updates_the_vault_destinations() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(50),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(50),
                msg: None,
            },
        ];

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations.clone()),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_ne!(updated_vault.destinations, vault.destinations);
        assert_eq!(updated_vault.destinations, destinations);
    }

    #[test]
    fn sets_the_vault_destination_to_owner_when_update_list_is_empty() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(vec![]),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_ne!(updated_vault.destinations, vault.destinations);
        assert_eq!(
            updated_vault.destinations,
            vec![Destination {
                address: vault.owner,
                allocation: Decimal::percent(100),
                msg: None,
            }]
        );
    }

    #[test]
    fn updates_slippage_tolerance() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let slippage_tolerance = Decimal::percent(1);

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            Some(slippage_tolerance),
            None,
            None,
            None,
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(updated_vault.slippage_tolerance, slippage_tolerance);
    }

    #[test]
    fn updates_minimum_receive_amount() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let minimum_receive_amount = Some(Uint128::new(12387));

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            None,
            minimum_receive_amount,
            None,
            None,
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(updated_vault.minimum_receive_amount, minimum_receive_amount);
    }

    #[test]
    fn updates_time_interval() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let time_interval = TimeInterval::Custom {
            seconds: 31271632321,
        };

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            None,
            None,
            None,
            Some(time_interval.clone()),
            None,
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(updated_vault.time_interval, time_interval);
    }
}
