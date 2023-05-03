use crate::error::ContractError;
use crate::helpers::validation::{
    assert_address_is_valid, assert_contract_is_not_paused,
    assert_destination_allocations_add_up_to_one, assert_destination_callback_addresses_are_valid,
    assert_destinations_limit_is_not_breached, assert_exactly_one_asset,
    assert_no_destination_allocations_are_zero, assert_pair_exists_for_denoms,
    assert_swap_adjusment_and_performance_assessment_strategies_are_compatible,
    assert_swap_amount_is_greater_than_50000, assert_target_start_time_is_in_future,
    assert_time_interval_is_valid,
};
use crate::helpers::vault::get_risk_weighted_average_model_id;
use crate::msg::ExecuteMsg;
use crate::state::cache::{VaultCache, VAULT_CACHE};
use crate::state::config::get_config;
use crate::state::events::create_event;
use crate::state::pairs::find_pair;
use crate::state::triggers::save_trigger;
use crate::state::vaults::save_vault;
use crate::types::destination::Destination;
use crate::types::event::{EventBuilder, EventData};
use crate::types::performance_assessment_strategy::{
    PerformanceAssessmentStrategy, PerformanceAssessmentStrategyParams,
};
use crate::types::position_type::PositionType;
use crate::types::swap_adjustment_strategy::{
    SwapAdjustmentStrategy, SwapAdjustmentStrategyParams,
};
use crate::types::time_interval::TimeInterval;
use crate::types::trigger::{Trigger, TriggerConfiguration};
use crate::types::vault::{VaultBuilder, VaultStatus};
use cosmwasm_std::{to_binary, Addr, Coin, Decimal, SubMsg, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Timestamp, Uint128, Uint64};

pub fn create_vault_handler(
    deps: DepsMut,
    env: Env,
    info: &MessageInfo,
    owner: Addr,
    label: Option<String>,
    mut destinations: Vec<Destination>,
    target_denom: String,
    position_type: Option<PositionType>,
    slippage_tolerance: Option<Decimal>,
    minimum_receive_amount: Option<Uint128>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
    performance_assessment_strategy_params: Option<PerformanceAssessmentStrategyParams>,
    swap_adjustment_strategy_params: Option<SwapAdjustmentStrategyParams>,
) -> Result<Response, ContractError> {
    assert_contract_is_not_paused(deps.storage)?;
    assert_address_is_valid(deps.as_ref(), owner.clone(), "owner")?;
    assert_exactly_one_asset(info.funds.clone())?;
    assert_swap_amount_is_greater_than_50000(swap_amount)?;
    assert_destinations_limit_is_not_breached(&destinations)?;
    assert_time_interval_is_valid(&time_interval)?;
    assert_pair_exists_for_denoms(
        deps.as_ref(),
        info.funds[0].denom.clone(),
        target_denom.clone(),
    )?;
    assert_swap_adjusment_and_performance_assessment_strategies_are_compatible(
        &swap_adjustment_strategy_params,
        &performance_assessment_strategy_params,
    )?;

    if let Some(target_time) = target_start_time_utc_seconds {
        assert_target_start_time_is_in_future(
            env.block.time,
            Timestamp::from_seconds(target_time.u64()),
        )?;
    }

    if destinations.is_empty() {
        destinations.push(Destination {
            allocation: Decimal::percent(100),
            address: owner.clone(),
            msg: None,
        });
    }

    assert_destination_callback_addresses_are_valid(deps.as_ref(), &destinations)?;
    assert_no_destination_allocations_are_zero(&destinations)?;
    assert_destination_allocations_add_up_to_one(&destinations)?;

    let config = get_config(deps.storage)?;

    let swap_denom = info.funds[0].denom.clone();

    let pair = find_pair(deps.storage, &[swap_denom.clone(), target_denom.clone()])?;

    let swap_adjustment_strategy =
        swap_adjustment_strategy_params
            .clone()
            .map(|params| match params {
                SwapAdjustmentStrategyParams::RiskWeightedAverage { base_denom } => {
                    SwapAdjustmentStrategy::RiskWeightedAverage {
                        model_id: get_risk_weighted_average_model_id(
                            &env.block.time,
                            &info.funds[0],
                            &swap_amount,
                            &time_interval,
                        ),
                        base_denom,
                        position_type: pair.position_type(swap_denom.clone()),
                    }
                }
                SwapAdjustmentStrategyParams::WeightedScale {
                    base_receive_amount,
                    multiplier,
                    increase_only,
                } => SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount,
                    multiplier,
                    increase_only,
                },
            });

    let performance_assessment_strategy = swap_adjustment_strategy_params.map(|_| {
        PerformanceAssessmentStrategy::CompareToStandardDca {
            swapped_amount: Coin::new(0, swap_denom.clone()),
            received_amount: Coin::new(0, target_denom.clone()),
        }
    });

    let escrow_level = performance_assessment_strategy
        .clone()
        .map_or(Decimal::zero(), |_| {
            config.risk_weighted_average_escrow_level
        });

    let vault_builder = VaultBuilder {
        owner,
        label,
        destinations,
        created_at: env.block.time,
        status: VaultStatus::Scheduled,
        target_denom: target_denom.clone(),
        swap_amount,
        position_type,
        slippage_tolerance,
        minimum_receive_amount,
        balance: info.funds[0].clone(),
        time_interval,
        started_at: None,
        escrow_level,
        deposited_amount: info.funds[0].clone(),
        swapped_amount: Coin::new(0, swap_denom),
        received_amount: Coin::new(0, target_denom.clone()),
        escrowed_amount: Coin::new(0, target_denom),
        swap_adjustment_strategy,
        performance_assessment_strategy,
    };

    let vault = save_vault(deps.storage, vault_builder)?;

    VAULT_CACHE.save(deps.storage, &VaultCache { vault_id: vault.id })?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultFundsDeposited {
                amount: info.funds[0].clone(),
            },
        ),
    )?;

    let mut response = Response::new()
        .add_attribute("create_vault", "true")
        .add_attribute("vault_id", vault.id)
        .add_attribute("deposited_amount", vault.balance.to_string());

    save_trigger(
        deps.storage,
        Trigger {
            vault_id: vault.id,
            configuration: TriggerConfiguration::Time {
                target_time: match target_start_time_utc_seconds {
                    Some(time) => Timestamp::from_seconds(time.u64()),
                    None => env.block.time,
                },
            },
        },
    )?;

    if target_start_time_utc_seconds.is_none() {
        response = response.add_submessage(SubMsg::new(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::ExecuteTrigger {
                trigger_id: vault.id,
            })
            .unwrap(),
            funds: vec![],
        }));
    }

    Ok(response)
}

#[cfg(test)]
mod create_vault_tests {
    use super::*;
    use crate::handlers::create_pair::create_pair_handler;
    use crate::handlers::get_events_by_resource_id::get_events_by_resource_id_handler;
    use crate::handlers::get_vault::get_vault_handler;
    use crate::msg::ExecuteMsg;
    use crate::state::config::{get_config, update_config};
    use crate::tests::helpers::instantiate_contract;
    use crate::tests::mocks::{
        calc_mock_dependencies, ADMIN, DENOM_STAKE, DENOM_UOSMO, USER, VALIDATOR,
    };
    use crate::types::config::Config;
    use crate::types::destination::Destination;
    use crate::types::event::{EventBuilder, EventData};
    use crate::types::pair::Pair;
    use crate::types::swap_adjustment_strategy::SwapAdjustmentStrategy;
    use crate::types::time_interval::TimeInterval;
    use crate::types::trigger::TriggerConfiguration;
    use crate::types::vault::{Vault, VaultStatus};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{to_binary, Addr, Coin, Decimal, SubMsg, Timestamp, Uint128, WasmMsg};

    #[test]
    fn with_no_assets_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(10000),
            TimeInterval::Daily,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: received 0 denoms but required exactly 1"
        );
    }

    #[test]
    fn with_multiple_assets_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(
            USER,
            &[Coin::new(10000, DENOM_UOSMO), Coin::new(10000, DENOM_STAKE)],
        );

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(10000),
            TimeInterval::Daily,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: received 2 denoms but required exactly 1"
        );
    }

    #[test]
    fn with_non_existent_pair_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: swapping stake to uosmo not supported"
        );
    }

    #[test]
    fn with_destination_allocations_less_than_100_percent_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), admin_info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            admin_info.clone(),
            pair.base_denom.clone(),
            pair.quote_denom.clone(),
            pair.route.clone(),
        )
        .unwrap();

        let user_info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &user_info,
            user_info.sender.clone(),
            None,
            vec![Destination {
                allocation: Decimal::percent(50),
                address: Addr::unchecked(USER),
                msg: None,
            }],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
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
    fn with_destination_allocation_equal_to_zero_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), admin_info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            admin_info.clone(),
            pair.base_denom.clone(),
            pair.quote_denom.clone(),
            pair.route.clone(),
        )
        .unwrap();

        let user_info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &user_info,
            user_info.sender.clone(),
            None,
            vec![
                Destination {
                    allocation: Decimal::percent(100),
                    address: Addr::unchecked(USER),
                    msg: None,
                },
                Destination {
                    allocation: Decimal::percent(0),
                    address: Addr::unchecked("other"),
                    msg: None,
                },
            ],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
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
    fn with_more_than_10_destination_allocations_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            (0..20)
                .into_iter()
                .map(|i| Destination {
                    allocation: Decimal::percent(5),
                    address: Addr::unchecked(format!("destination-{}", i)),
                    msg: None,
                })
                .collect(),
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
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
    fn with_swap_amount_less_than_50000_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(10000),
            TimeInterval::Daily,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: swap amount must be greater than 50000"
        );
    }

    #[test]
    fn when_contract_is_paused_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let config = get_config(deps.as_ref().storage).unwrap();

        update_config(
            deps.as_mut().storage,
            Config {
                paused: true,
                ..config
            },
        )
        .unwrap();

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Error: contract is paused")
    }

    #[test]
    fn with_time_trigger_with_target_time_in_the_past_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), admin_info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            admin_info.clone(),
            pair.base_denom.clone(),
            pair.quote_denom.clone(),
            pair.route.clone(),
        )
        .unwrap();

        let user_info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        let err = create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &user_info,
            user_info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.minus_seconds(10).seconds().into()),
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: target_start_time_utc_seconds must be some time in the future"
        );
    }

    #[test]
    fn with_invalid_custom_time_interval_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let err = create_vault_handler(
            deps.as_mut(),
            env,
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Custom { seconds: 23 },
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: custom time interval must be at least 60 seconds"
        );
    }

    #[test]
    fn with_no_swap_adjustment_stratgey_and_performance_assessment_strategy_fails() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom.clone(),
            pair.quote_denom.clone(),
            pair.route.clone(),
        )
        .unwrap();

        let swap_amount = Uint128::new(100000);
        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        let err = create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            swap_amount,
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(PerformanceAssessmentStrategyParams::CompareToStandardDca),
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: incompatible swap adjustment and performance assessment strategies"
        );
    }

    #[test]
    fn with_swap_adjustment_stratgey_and_no_performance_assessment_strategy_fails() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom.clone(),
            pair.quote_denom.clone(),
            pair.route.clone(),
        )
        .unwrap();

        let swap_amount = Uint128::new(100000);
        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        let err = create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            swap_amount,
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            None,
            Some(SwapAdjustmentStrategyParams::default()),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: incompatible swap adjustment and performance assessment strategies"
        );
    }

    #[test]
    fn should_create_vault() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom.clone(),
            pair.quote_denom.clone(),
            pair.route.clone(),
        )
        .unwrap();

        let swap_amount = Uint128::new(100000);
        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            swap_amount,
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            None,
            None,
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(
            vault,
            Vault {
                minimum_receive_amount: None,
                label: None,
                id: Uint128::new(1),
                owner: info.sender,
                destinations: vec![Destination::default()],
                created_at: env.block.time,
                status: VaultStatus::Scheduled,
                time_interval: TimeInterval::Daily,
                balance: info.funds[0].clone(),
                slippage_tolerance: None,
                swap_amount,
                target_denom: DENOM_UOSMO.to_string(),
                started_at: None,
                deposited_amount: info.funds[0].clone(),
                escrow_level: Decimal::zero(),
                swapped_amount: Coin::new(0, DENOM_STAKE.to_string()),
                received_amount: Coin::new(0, DENOM_UOSMO.to_string()),
                escrowed_amount: Coin::new(0, DENOM_UOSMO.to_string()),
                swap_adjustment_strategy: None,
                performance_assessment_strategy: None,
                trigger: Some(TriggerConfiguration::Time {
                    target_time: Timestamp::from_seconds(env.block.time.plus_seconds(10).seconds()),
                }),
            }
        );
    }

    #[test]
    fn should_publish_deposit_event() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            None,
            None,
        )
        .unwrap();

        let events =
            get_events_by_resource_id_handler(deps.as_ref(), Uint128::one(), None, None, None)
                .unwrap()
                .events;

        assert!(events.contains(
            &EventBuilder::new(
                Uint128::one(),
                env.block,
                EventData::DcaVaultFundsDeposited {
                    amount: info.funds[0].clone()
                },
            )
            .build(1),
        ))
    }

    #[test]
    fn for_different_owner_should_succeed() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        let owner = Addr::unchecked(USER);
        info = mock_info(ADMIN, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            owner,
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            None,
            None,
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(vault.owner, Addr::unchecked(USER));
    }

    #[test]
    fn with_multiple_destinations_should_succeed() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        let destinations = vec![
            Destination {
                allocation: Decimal::percent(50),
                address: env.contract.address.clone(),
                msg: Some(
                    to_binary(&ExecuteMsg::ZDelegate {
                        delegator_address: Addr::unchecked("dest-1"),
                        validator_address: Addr::unchecked(VALIDATOR),
                    })
                    .unwrap(),
                ),
            },
            Destination {
                allocation: Decimal::percent(50),
                address: env.contract.address.clone(),
                msg: Some(
                    to_binary(&ExecuteMsg::ZDelegate {
                        delegator_address: Addr::unchecked("dest-2"),
                        validator_address: Addr::unchecked(VALIDATOR),
                    })
                    .unwrap(),
                ),
            },
        ];

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            destinations.clone(),
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            None,
            None,
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(vault.destinations, destinations);
    }

    #[test]
    fn should_create_swap_adjustment_strategy() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(PerformanceAssessmentStrategyParams::CompareToStandardDca),
            Some(SwapAdjustmentStrategyParams::default()),
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(
            vault.swap_adjustment_strategy,
            Some(SwapAdjustmentStrategy::default())
        );
    }

    #[test]
    fn should_create_performance_assessment_strategy() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(PerformanceAssessmentStrategyParams::CompareToStandardDca),
            Some(SwapAdjustmentStrategyParams::default()),
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(
            vault.performance_assessment_strategy,
            Some(PerformanceAssessmentStrategy::CompareToStandardDca {
                swapped_amount: Coin::new(0, vault.balance.denom),
                received_amount: Coin::new(0, DENOM_UOSMO),
            })
        );
    }

    #[test]
    fn with_large_deposit_should_select_longer_duration_model() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(1000000000, DENOM_STAKE)]);

        create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            Some(env.block.time.plus_seconds(10).seconds().into()),
            Some(PerformanceAssessmentStrategyParams::CompareToStandardDca),
            Some(SwapAdjustmentStrategyParams::default()),
        )
        .unwrap();

        let vault = get_vault_handler(deps.as_ref(), Uint128::one())
            .unwrap()
            .vault;

        assert_eq!(
            vault
                .swap_adjustment_strategy
                .map(|strategy| match strategy {
                    SwapAdjustmentStrategy::RiskWeightedAverage { model_id, .. } => model_id,
                    _ => panic!("unexpected swap adjustment strategy"),
                }),
            Some(90)
        );
    }

    #[test]
    fn with_no_target_time_should_execute_vault() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let mut info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom,
            pair.quote_denom,
            pair.route,
        )
        .unwrap();

        info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

        let response = create_vault_handler(
            deps.as_mut(),
            env.clone(),
            &info,
            info.sender.clone(),
            None,
            vec![],
            DENOM_UOSMO.to_string(),
            None,
            None,
            None,
            Uint128::new(100000),
            TimeInterval::Daily,
            None,
            Some(PerformanceAssessmentStrategyParams::CompareToStandardDca),
            Some(SwapAdjustmentStrategyParams::default()),
        )
        .unwrap();

        assert!(response.messages.contains(&SubMsg::new(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::ExecuteTrigger {
                trigger_id: Uint128::one()
            })
            .unwrap()
        })));
    }
}
