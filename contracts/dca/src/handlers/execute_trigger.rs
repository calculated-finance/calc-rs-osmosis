use crate::constants::AFTER_SWAP_REPLY_ID;
use crate::error::ContractError;
use crate::helpers::price::query_belief_price;
use crate::helpers::swaps::create_osmosis_swap_message;
use crate::helpers::time::get_next_target_time;
use crate::helpers::validation::{assert_contract_is_not_paused, assert_target_time_is_in_past};
use crate::helpers::vault::{
    get_swap_amount, price_threshold_exceeded, simulate_standard_dca_execution,
};
use crate::msg::ExecuteMsg;
use crate::state::cache::{SwapCache, VaultCache, SWAP_CACHE, VAULT_CACHE};
use crate::state::events::create_event;
use crate::state::pairs::find_pair;
use crate::state::triggers::{delete_trigger, save_trigger};
use crate::state::vaults::{get_vault, update_vault};
use crate::types::event::{EventBuilder, EventData, ExecutionSkippedReason};
use crate::types::swap_adjustment_strategy::SwapAdjustmentStrategy;
use crate::types::trigger::{Trigger, TriggerConfiguration};
use crate::types::vault::{Vault, VaultStatus};
use cosmwasm_std::{to_binary, ReplyOn, SubMsg, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, Response, Uint128};

pub fn execute_trigger_handler(
    deps: DepsMut,
    env: Env,
    trigger_id: Uint128,
) -> Result<Response, ContractError> {
    assert_contract_is_not_paused(deps.storage)?;

    let mut vault = get_vault(deps.storage, trigger_id)?;
    let mut response = Response::new()
        .add_attribute("execute_vault", "true")
        .add_attribute("vault_id", vault.id);

    delete_trigger(deps.storage, vault.id)?;

    if vault.is_cancelled() {
        return Err(ContractError::CustomError {
            val: format!(
                "vault with id {} is cancelled, and is not available for execution",
                vault.id
            ),
        });
    }

    if vault.trigger.is_none() {
        return Err(ContractError::CustomError {
            val: format!(
                "vault with id {} has no trigger attached, and is not available for execution",
                vault.id
            ),
        });
    }

    match vault
        .trigger
        .clone()
        .unwrap_or_else(|| panic!("trigger for vault id {}", vault.id))
    {
        TriggerConfiguration::Time { target_time } => {
            assert_target_time_is_in_past(env.block.time, target_time)?;
        }
    }

    if vault.is_scheduled() {
        vault = Vault {
            status: VaultStatus::Active,
            started_at: Some(env.block.time),
            ..vault
        };
    }

    update_vault(deps.storage, &vault)?;

    let pair = find_pair(deps.storage, &vault.denoms())?;

    let belief_price = query_belief_price(&deps.querier, &pair, vault.get_swap_denom())?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.to_owned(),
            EventData::DcaVaultExecutionTriggered {
                base_denom: pair.base_denom.clone(),
                quote_denom: pair.quote_denom.clone(),
                asset_price: belief_price,
            },
        ),
    )?;

    response = response.add_attribute("belief_price", belief_price.to_string());

    if let Some(SwapAdjustmentStrategy::RiskWeightedAverage { .. }) = vault.swap_adjustment_strategy
    {
        (vault, response) = simulate_standard_dca_execution(
            response,
            &deps.querier,
            deps.storage,
            &env,
            vault,
            belief_price,
        )?;
    }

    let should_execute_again = vault.is_active()
        || vault.performance_assessment_strategy.clone().map_or(
            false,
            |performance_assessment_strategy| {
                performance_assessment_strategy
                    .standard_dca_balance(vault.deposited_amount.clone())
                    .amount
                    > Uint128::zero()
            },
        );

    if should_execute_again {
        save_trigger(
            deps.storage,
            Trigger {
                vault_id: vault.id,
                configuration: TriggerConfiguration::Time {
                    target_time: get_next_target_time(
                        env.block.time,
                        match vault.trigger {
                            Some(TriggerConfiguration::Time { target_time }) => target_time,
                            _ => env.block.time,
                        },
                        vault.time_interval.clone(),
                    ),
                },
            },
        )?;
    } else {
        if vault.should_not_continue() && vault.escrowed_amount.amount > Uint128::zero() {
            response = response.add_submessage(SubMsg::new(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::DisburseEscrow { vault_id: vault.id })?,
                funds: vec![],
            }));
        }

        return Ok(response);
    }

    if vault.is_inactive() {
        return Ok(response);
    }

    let swap_amount = get_swap_amount(&deps.as_ref(), &env, &vault)?;

    if swap_amount.amount.is_zero() {
        create_event(
            deps.storage,
            EventBuilder::new(
                vault.id,
                env.block,
                EventData::DcaVaultExecutionSkipped {
                    reason: ExecutionSkippedReason::SwapAmountAdjustedToZero,
                },
            ),
        )?;

        response = response.add_attribute("execution_skipped", "swap_amount_adjusted_to_zero");

        return Ok(response);
    }

    if price_threshold_exceeded(
        swap_amount.amount,
        vault.minimum_receive_amount,
        belief_price,
    )? {
        create_event(
            deps.storage,
            EventBuilder::new(
                vault.id,
                env.block,
                EventData::DcaVaultExecutionSkipped {
                    reason: ExecutionSkippedReason::PriceThresholdExceeded {
                        price: belief_price,
                    },
                },
            ),
        )?;

        response = response.add_attribute("execution_skipped", "price_threshold_exceeded");

        return Ok(response);
    };

    VAULT_CACHE.save(deps.storage, &VaultCache { vault_id: vault.id })?;

    SWAP_CACHE.save(
        deps.storage,
        &SwapCache {
            swap_denom_balance: deps
                .querier
                .query_balance(&env.contract.address, vault.get_swap_denom())?,
            receive_denom_balance: deps
                .querier
                .query_balance(&env.contract.address, vault.target_denom)?,
        },
    )?;

    Ok(response.add_submessage(create_osmosis_swap_message(
        &deps.querier,
        &env,
        &pair,
        swap_amount,
        vault.slippage_tolerance,
        Some(AFTER_SWAP_REPLY_ID),
        Some(ReplyOn::Always),
    )?))
}

#[cfg(test)]
mod execute_trigger_tests {
    use super::*;
    use crate::constants::{ONE, ONE_MICRON, SWAP_FEE_RATE, TEN, TWO_MICRONS};
    use crate::handlers::get_events_by_resource_id::get_events_by_resource_id_handler;
    use crate::helpers::fees::{get_delegation_fee_rate, get_swap_fee_rate};
    use crate::helpers::vault::get_swap_amount;
    use crate::msg::ExecuteMsg;
    use crate::state::config::update_config;
    use crate::state::swap_adjustments::update_swap_adjustment;
    use crate::state::triggers::delete_trigger;
    use crate::state::vaults::get_vault;
    use crate::tests::helpers::{instantiate_contract, setup_vault};
    use crate::tests::mocks::{calc_mock_dependencies, ADMIN, DENOM_STAKE, DENOM_UOSMO};
    use crate::types::config::Config;
    use crate::types::event::{Event, EventData, ExecutionSkippedReason};
    use crate::types::performance_assessment_strategy::PerformanceAssessmentStrategy;
    use crate::types::position_type::PositionType;
    use crate::types::swap_adjustment_strategy::{BaseDenom, SwapAdjustmentStrategy};
    use crate::types::trigger::TriggerConfiguration;
    use crate::types::vault::{Vault, VaultStatus};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{to_binary, Coin, Decimal, ReplyOn, StdError, SubMsg, Uint128, WasmMsg};
    use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceResponse;
    use osmosis_std::types::osmosis::poolmanager::v1beta1::{
        EstimateSwapExactAmountInResponse, MsgSwapExactAmountIn, SwapAmountInRoute,
    };
    use std::str::FromStr;

    #[test]
    fn when_contract_is_paused_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        update_config(
            deps.as_mut().storage,
            Config {
                paused: true,
                ..Config::default()
            },
        )
        .unwrap();

        let err = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap_err();

        assert_eq!(err.to_string(), "Error: contract is paused");
    }

    #[test]
    fn when_vault_is_cancelled_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Cancelled,
                ..Vault::default()
            },
        );

        let err = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: vault with id 0 is cancelled, and is not available for execution"
        );
    }

    #[test]
    fn when_vault_is_cancelled_should_delete_trigger() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Cancelled,
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap_err();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert!(vault.trigger.is_some());
        assert_eq!(updated_vault.trigger, None);
    }

    #[test]
    fn when_no_trigger_exists_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                ..Vault::default()
            },
        );

        delete_trigger(deps.as_mut().storage, vault.id).unwrap();

        let err = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: vault with id 0 has no trigger attached, and is not available for execution"
        );
    }

    #[test]
    fn when_trigger_is_not_ready_to_fire_should_fail() {
        let mut deps = calc_mock_dependencies();
        let mut env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        env.block.time = env.block.time.minus_seconds(10);

        let err = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: trigger execution time has not yet elapsed"
        );
    }

    #[test]
    fn should_make_scheduled_vault_active() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Scheduled,
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(vault.status, VaultStatus::Scheduled);
        assert_eq!(updated_vault.status, VaultStatus::Active);
    }

    #[test]
    fn should_set_scheduled_vault_start_time() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Scheduled,
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(vault.started_at, None);
        assert_eq!(updated_vault.started_at, Some(env.block.time));
    }

    #[test]
    fn publishes_execution_triggered_event() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        let pair = find_pair(deps.as_ref().storage, &vault.denoms()).unwrap();

        assert!(events.contains(&Event {
            id: 1,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultExecutionTriggered {
                base_denom: pair.base_denom,
                quote_denom: pair.quote_denom,
                asset_price: Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap()
            }
        }));
    }

    #[test]
    fn for_non_standard_dca_should_simulate_execution() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        let fee_rate = get_swap_fee_rate(deps.as_mut().storage, &vault).unwrap()
            + get_delegation_fee_rate(deps.as_mut().storage, &vault).unwrap();

        let received_amount_before_fee = vault.swap_amount * Decimal::one();
        let fee_amount = received_amount_before_fee * fee_rate;
        let received_amount_after_fee = received_amount_before_fee - fee_amount;

        let performance_assessment_strategy =
            updated_vault.performance_assessment_strategy.unwrap();

        assert_eq!(
            performance_assessment_strategy.standard_dca_swapped_amount(),
            Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
        );
        assert_eq!(
            performance_assessment_strategy.standard_dca_received_amount(),
            Coin::new(received_amount_after_fee.into(), vault.target_denom)
        );
    }

    #[test]
    fn with_finished_compare_to_standard_dca_performance_assessment_should_not_simulate_execution()
    {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new((TEN * Decimal::percent(5)).into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                        received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(
            updated_vault.swap_adjustment_strategy.unwrap(),
            vault.swap_adjustment_strategy.unwrap()
        );
    }

    #[test]
    fn with_swap_adjusted_to_zero_should_publish_execution_skipped_event() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount: ONE,
                    multiplier: Decimal::percent(500),
                    increase_only: false,
                }),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: "3".to_string(),
            }),
            _ => Err(StdError::generic_err("message not customised")),
        });

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert!(events.contains(&Event {
            id: 2,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::SwapAmountAdjustedToZero
            }
        }));
    }

    #[test]
    fn with_swap_adjusted_to_zero_should_not_send_swap_message() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::WeightedScale {
                    base_receive_amount: ONE,
                    multiplier: Decimal::percent(500),
                    increase_only: false,
                }),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: "3".to_string(),
            }),
            _ => Err(StdError::generic_err("message not customised")),
        });

        let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        assert!(response.messages.is_empty());
    }

    #[test]
    fn with_rwa_swap_adjustment_strategy_should_adjust_swap_amount() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new(0, DENOM_STAKE),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let swap_adjustment = Decimal::percent(150);

        [PositionType::Enter, PositionType::Exit]
            .into_iter()
            .for_each(|position_type| {
                update_swap_adjustment(
                    deps.as_mut().storage,
                    SwapAdjustmentStrategy::RiskWeightedAverage {
                        model_id: 30,
                        base_denom: BaseDenom::Bitcoin,
                        position_type,
                    },
                    swap_adjustment,
                    env.block.time,
                )
                .unwrap();
            });

        let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        assert!(response.messages.contains(&SubMsg {
            id: AFTER_SWAP_REPLY_ID,
            msg: MsgSwapExactAmountIn {
                sender: env.contract.address.to_string(),
                token_in: Some(
                    Coin::new(
                        (vault.swap_amount * swap_adjustment).into(),
                        vault.get_swap_denom()
                    )
                    .clone()
                    .into()
                ),
                token_out_min_amount: Uint128::one().to_string(),
                routes: vec![SwapAmountInRoute {
                    pool_id: 3,
                    token_out_denom: vault.target_denom,
                }],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }))
    }

    #[test]
    fn with_rwa_swap_adjustment_strategy_and_exceeded_slippage_tolerance_should_simulate_skipped_execution(
    ) {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                slippage_tolerance: Some(Decimal::percent(1)),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                to_binary(&EstimateSwapExactAmountInResponse {
                    token_out_amount: (ONE / TWO_MICRONS).to_string(),
                })
            }
            _ => Err(StdError::generic_err("message not supported")),
        });

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(
            updated_vault.swap_adjustment_strategy.unwrap(),
            SwapAdjustmentStrategy::default()
        );
    }

    #[test]
    fn with_rwa_swap_adjustment_strategy_and_exceeded_price_threshold_should_publish_execution_skipped_event(
    ) {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_amount: ONE,
                minimum_receive_amount: Some(ONE + ONE),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert!(events.contains(&Event {
            id: 2,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::SimulatedDcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::PriceThresholdExceeded {
                    price: Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap()
                },
            }
        }));
    }

    #[test]
    fn with_rwa_swap_adjustment_strategy_and_exceeded_slippage_tolerance_should_publish_execution_skipped_event(
    ) {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                slippage_tolerance: Some(Decimal::percent(1)),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                to_binary(&EstimateSwapExactAmountInResponse {
                    token_out_amount: (ONE / TWO_MICRONS).to_string(),
                })
            }
            _ => Err(StdError::generic_err("message not supported")),
        });

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert!(events.contains(&Event {
            id: 2,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::SimulatedDcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::SlippageToleranceExceeded
            }
        }));
    }

    #[test]
    fn for_inactive_vault_with_active_performance_assessment_should_simulate_execution() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                balance: Coin::new(TEN.into(), DENOM_UOSMO),
                performance_assessment_strategy: Some(PerformanceAssessmentStrategy::default()),
                escrow_level: Decimal::percent(5),
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        let fee_rate = get_swap_fee_rate(deps.as_ref().storage, &vault).unwrap()
            + get_delegation_fee_rate(deps.as_ref().storage, &vault).unwrap();

        let received_amount_before_fee = vault.swap_amount;
        let fee_amount = received_amount_before_fee * fee_rate;
        let received_amount_after_fee = received_amount_before_fee - fee_amount;

        let performance_assessment_strategy =
            updated_vault.performance_assessment_strategy.unwrap();

        assert_eq!(
            performance_assessment_strategy.standard_dca_swapped_amount(),
            Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
        );
        assert_eq!(
            performance_assessment_strategy.standard_dca_received_amount(),
            Coin::new(received_amount_after_fee.into(), vault.target_denom)
        );
    }

    #[test]
    fn for_inactive_vault_with_finished_performance_assessment_should_disburse_escrow() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                balance: Coin::new(0, DENOM_UOSMO),
                escrow_level: Decimal::percent(5),
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new(ONE.into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                        received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                to_binary(&EstimateSwapExactAmountInResponse {
                    token_out_amount: (ONE / TWO_MICRONS).to_string(),
                })
            }
            _ => Err(StdError::generic_err("message not supported")),
        });

        let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        assert!(response.messages.contains(&SubMsg::new(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::DisburseEscrow { vault_id: vault.id }).unwrap(),
            funds: vec![],
        })));
    }

    #[test]
    fn for_inactive_vault_with_unfinished_performance_assessment_should_not_disburse_escrow() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                balance: Coin::new(0, DENOM_UOSMO),
                escrow_level: Decimal::percent(5),
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new(ONE_MICRON.into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                        received_amount: Coin::new(ONE.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        assert!(response.messages.is_empty());
    }

    #[test]
    fn for_active_vault_should_create_a_new_trigger() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        let old_target_time = match vault.trigger.unwrap() {
            TriggerConfiguration::Time { target_time } => target_time,
        };

        let new_target_time = match updated_vault.trigger.unwrap() {
            TriggerConfiguration::Time { target_time } => target_time,
        };

        assert_eq!(old_target_time.seconds(), env.block.time.seconds());
        assert_eq!(
            new_target_time.seconds(),
            env.block.time.plus_seconds(24 * 60 * 60).seconds()
        );
    }

    #[test]
    fn for_scheduled_vault_should_create_a_new_trigger() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Scheduled,
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        let old_target_time = match vault.trigger.unwrap() {
            TriggerConfiguration::Time { target_time } => target_time,
        };

        let new_target_time = match updated_vault.trigger.unwrap() {
            TriggerConfiguration::Time { target_time } => target_time,
        };

        assert_eq!(old_target_time.seconds(), env.block.time.seconds());
        assert_eq!(
            new_target_time.seconds(),
            env.block.time.plus_seconds(24 * 60 * 60).seconds()
        );
    }

    #[test]
    fn for_inactive_vault_should_not_create_a_new_trigger() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert!(vault.trigger.is_some());
        assert!(updated_vault.trigger.is_none(),);
    }

    #[test]
    fn for_inactive_vault_with_active_performance_asssessment_should_create_a_new_trigger() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                escrow_level: Decimal::percent(5),
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new(ONE_MICRON.into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                        received_amount: Coin::new(ONE.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        deps.querier.update_stargate(|path, _| match path {
            "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                to_binary(&EstimateSwapExactAmountInResponse {
                    token_out_amount: (ONE / TWO_MICRONS).to_string(),
                })
            }
            _ => Err(StdError::generic_err("message not supported")),
        });

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        let old_target_time = match vault.trigger.unwrap() {
            TriggerConfiguration::Time { target_time } => target_time,
        };

        let new_target_time = match updated_vault.trigger.unwrap() {
            TriggerConfiguration::Time { target_time } => target_time,
        };

        assert_eq!(old_target_time.seconds(), env.block.time.seconds());
        assert_eq!(
            new_target_time.seconds(),
            env.block.time.plus_seconds(24 * 60 * 60).seconds()
        );
    }

    #[test]
    fn for_inactive_vault_with_finished_performance_assessment_should_not_create_a_new_trigger() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Inactive,
                escrow_level: Decimal::percent(5),
                deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_amount: Coin::new((TEN * Decimal::percent(5)).into(), DENOM_STAKE),
                performance_assessment_strategy: Some(
                    PerformanceAssessmentStrategy::CompareToStandardDca {
                        swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                        received_amount: Coin::new(TEN.into(), DENOM_STAKE),
                    },
                ),
                swap_adjustment_strategy: Some(SwapAdjustmentStrategy::default()),
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert!(vault.trigger.is_some());
        assert!(updated_vault.trigger.is_none());
    }

    #[test]
    fn should_create_swap_message() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        assert!(response.messages.contains(&SubMsg {
            id: AFTER_SWAP_REPLY_ID,
            msg: MsgSwapExactAmountIn {
                sender: env.contract.address.to_string(),
                token_in: Some(
                    Coin::new(vault.swap_amount.into(), vault.get_swap_denom())
                        .clone()
                        .into()
                ),
                token_out_min_amount: Uint128::one().to_string(),
                routes: vec![SwapAmountInRoute {
                    pool_id: 3,
                    token_out_denom: vault.target_denom,
                }],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }))
    }

    #[test]
    fn should_create_reduced_swap_message_when_balance_is_low() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                balance: Coin::new((ONE / TWO_MICRONS).into(), DENOM_UOSMO),
                swap_amount: ONE,
                ..Vault::default()
            },
        );

        let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        assert!(response.messages.contains(&SubMsg {
            id: AFTER_SWAP_REPLY_ID,
            msg: MsgSwapExactAmountIn {
                sender: env.contract.address.to_string(),
                token_in: Some(vault.balance.clone().into()),
                token_out_min_amount: Uint128::one().to_string(),
                routes: vec![SwapAmountInRoute {
                    pool_id: 3,
                    token_out_denom: vault.target_denom,
                }],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }))
    }

    #[test]
    fn should_create_swap_message_with_target_receive_amount_when_slippage_tolerance_set() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                slippage_tolerance: Some(Decimal::percent(1)),
                ..Vault::default()
            },
        );

        let belief_price = Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap();

        let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let token_out_min_amount = get_swap_amount(&deps.as_ref(), &env, &vault)
            .unwrap()
            .amount
            * (Decimal::one() / belief_price)
            * (Decimal::one() - vault.slippage_tolerance.unwrap());

        assert!(response.messages.contains(&SubMsg {
            id: AFTER_SWAP_REPLY_ID,
            msg: MsgSwapExactAmountIn {
                sender: env.contract.address.to_string(),
                token_in: Some(
                    get_swap_amount(&deps.as_ref(), &env, &vault)
                        .unwrap()
                        .into()
                ),
                token_out_min_amount: token_out_min_amount.to_string(),
                routes: vec![SwapAmountInRoute {
                    pool_id: 3,
                    token_out_denom: vault.target_denom,
                }],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }))
    }

    #[test]
    fn should_skip_execution_if_price_threshold_exceeded() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_amount: ONE,
                minimum_receive_amount: Some(ONE + ONE),
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let events = get_events_by_resource_id_handler(deps.as_ref(), vault.id, None, None, None)
            .unwrap()
            .events;

        assert!(events.contains(&Event {
            id: 2,
            resource_id: vault.id,
            timestamp: env.block.time,
            block_height: env.block.height,
            data: EventData::DcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::PriceThresholdExceeded {
                    price: Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap()
                }
            }
        }));
    }

    #[test]
    fn should_create_new_trigger_if_price_threshold_exceeded() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_amount: ONE,
                minimum_receive_amount: Some(ONE + ONE),
                ..Vault::default()
            },
        );

        execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        let old_target_time = match vault.trigger.unwrap() {
            TriggerConfiguration::Time { target_time } => target_time,
        };

        let new_target_time = match updated_vault.trigger.unwrap() {
            TriggerConfiguration::Time { target_time } => target_time,
        };

        assert_eq!(old_target_time.seconds(), env.block.time.seconds());
        assert_eq!(
            new_target_time.seconds(),
            env.block.time.plus_seconds(24 * 60 * 60).seconds()
        );
    }

    #[test]
    fn should_trigger_execution_if_price_threshold_not_exceeded() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let vault = setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                swap_amount: ONE,
                minimum_receive_amount: Some(ONE / Uint128::new(2)),
                ..Vault::default()
            },
        );

        let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

        assert!(response.messages.contains(&SubMsg {
            id: AFTER_SWAP_REPLY_ID,
            msg: MsgSwapExactAmountIn {
                sender: env.contract.address.to_string(),
                token_in: Some(
                    Coin::new(vault.swap_amount.into(), vault.get_swap_denom())
                        .clone()
                        .into()
                ),
                token_out_min_amount: Uint128::one().to_string(),
                routes: vec![SwapAmountInRoute {
                    pool_id: 3,
                    token_out_denom: vault.target_denom,
                }],
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Always,
        }))
    }
}
