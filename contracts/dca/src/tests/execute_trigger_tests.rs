use super::helpers::instantiate_contract;
use crate::constants::{ONE, ONE_MICRON, OSMOSIS_SWAP_FEE_RATE, TEN, TWO_MICRONS};
use crate::contract::AFTER_FIN_SWAP_REPLY_ID;
use crate::handlers::execute_trigger::execute_trigger_handler;
use crate::handlers::get_events_by_resource_id::get_events_by_resource_id;
use crate::helpers::fee_helpers::{get_delegation_fee_rate, get_swap_fee_rate};
use crate::helpers::vault_helpers::get_swap_amount;
use crate::msg::ExecuteMsg;
use crate::state::config::{update_config, Config};
use crate::state::swap_adjustments::update_swap_adjustments;
use crate::state::triggers::delete_trigger;
use crate::state::vaults::get_vault;
use crate::tests::helpers::setup_new_vault;
use crate::tests::mocks::{calc_mock_dependencies, ADMIN, DENOM_STAKE, DENOM_UOSMO};
use crate::types::dca_plus_config::DcaPlusConfig;
use crate::types::event::{Event, EventData, ExecutionSkippedReason};
use crate::types::position_type::PositionType;
use crate::types::trigger::TriggerConfiguration;
use crate::types::vault::{Vault, VaultStatus};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, Decimal, ReplyOn, StdError, SubMsg, Uint128, WasmMsg,
};
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

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

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

    let vault = setup_new_vault(
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

    let vault = setup_new_vault(
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

    let vault = setup_new_vault(
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

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

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

    let vault = setup_new_vault(
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

    let vault = setup_new_vault(
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

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(&Event {
        id: 1,
        resource_id: vault.id,
        timestamp: env.block.time,
        block_height: env.block.height,
        data: EventData::DcaVaultExecutionTriggered {
            base_denom: DENOM_UOSMO.to_string(),
            quote_denom: DENOM_STAKE.to_string(),
            asset_price: Decimal::one()
        }
    }));
}

#[test]
fn with_dca_plus_should_simulate_execution() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

    execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

    let fee_rate = get_swap_fee_rate(deps.as_mut().storage, &vault).unwrap()
        + get_delegation_fee_rate(deps.as_mut().storage, &vault).unwrap()
        + Decimal::from_str(OSMOSIS_SWAP_FEE_RATE).unwrap();

    let received_amount_before_fee = vault.swap_amount * Decimal::one();
    let fee_amount = received_amount_before_fee * fee_rate;
    let received_amount_after_fee = received_amount_before_fee - fee_amount;

    assert_eq!(
        updated_vault.dca_plus_config.unwrap(),
        DcaPlusConfig {
            standard_dca_swapped_amount: Coin::new(
                vault.swap_amount.into(),
                vault.get_swap_denom()
            ),
            standard_dca_received_amount: Coin::new(
                received_amount_after_fee.into(),
                vault.get_receive_denom()
            ),
            ..vault.dca_plus_config.unwrap()
        }
    );
}

#[test]
fn with_finished_dca_plus_should_not_simulate_execution() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        },
    );

    execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

    assert_eq!(
        updated_vault.dca_plus_config.unwrap(),
        vault.dca_plus_config.unwrap()
    );
}

#[test]
fn with_dca_plus_should_adjust_swap_amount() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let model_id = 30;

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            dca_plus_config: Some(DcaPlusConfig {
                model_id: 30,
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        },
    );

    let swap_adjustment = Decimal::percent(150);

    [PositionType::Enter, PositionType::Exit]
        .iter()
        .for_each(|position_type| {
            update_swap_adjustments(
                deps.as_mut().storage,
                position_type.clone(),
                vec![(model_id, swap_adjustment)],
                env.block.time,
            )
            .unwrap();
        });

    let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    assert!(response.messages.contains(&SubMsg {
        id: AFTER_FIN_SWAP_REPLY_ID,
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
                token_out_denom: vault.get_receive_denom(),
            }],
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Always,
    }))
}

#[test]
fn with_dca_plus_and_exceeded_slippage_tolerance_should_simulate_skipped_execution() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            slippage_tolerance: Some(Decimal::percent(1)),
            dca_plus_config: Some(DcaPlusConfig::default()),
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
        updated_vault.dca_plus_config.unwrap(),
        DcaPlusConfig {
            standard_dca_swapped_amount: Coin::new(0, vault.get_swap_denom()),
            standard_dca_received_amount: Coin::new(0, vault.get_receive_denom()),
            ..vault.dca_plus_config.unwrap()
        }
    );
}

#[test]
fn with_dca_plus_and_exceeded_price_threshold_should_publish_execution_skipped_event() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            swap_amount: ONE,
            minimum_receive_amount: Some(ONE + ONE),
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

    execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(&Event {
        id: 2,
        resource_id: vault.id,
        timestamp: env.block.time,
        block_height: env.block.height,
        data: EventData::SimulatedDcaVaultExecutionSkipped {
            reason: ExecutionSkippedReason::PriceThresholdExceeded {
                price: Decimal::one()
            },
        }
    }));
}

#[test]
fn with_dca_plus_and_exceeded_slippage_tolerance_should_publish_execution_skipped_event() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            slippage_tolerance: Some(Decimal::percent(1)),
            dca_plus_config: Some(DcaPlusConfig::default()),
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

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
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
fn for_inactive_vault_with_active_dca_plus_should_simulate_execution() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

    execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

    let fee_rate = get_swap_fee_rate(deps.as_ref().storage, &vault).unwrap()
        + get_delegation_fee_rate(deps.as_ref().storage, &vault).unwrap()
        + Decimal::from_str(OSMOSIS_SWAP_FEE_RATE).unwrap();

    let received_amount_before_fee = vault.swap_amount;
    let fee_amount = received_amount_before_fee * fee_rate;
    let received_amount_after_fee = received_amount_before_fee - fee_amount;

    assert_eq!(
        updated_vault.dca_plus_config.unwrap(),
        DcaPlusConfig {
            standard_dca_swapped_amount: Coin::new(
                vault.swap_amount.into(),
                vault.get_swap_denom()
            ),
            standard_dca_received_amount: Coin::new(
                received_amount_after_fee.into(),
                vault.get_receive_denom()
            ),
            ..vault.dca_plus_config.unwrap()
        }
    );
}

#[test]
fn for_inactive_vault_with_finished_dca_plus_should_disburse_escrow() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            balance: Coin::new(0, DENOM_UOSMO),
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                escrowed_balance: Coin::new(ONE.into(), DENOM_UOSMO),
                ..DcaPlusConfig::default()
            }),
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

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::DisburseEscrow { vault_id: vault.id }).unwrap(),
            funds: vec![],
        }))));
}

#[test]
fn for_inactive_vault_with_unfinished_dca_plus_should_not_disburse_escrow() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            balance: Coin::new(0, DENOM_UOSMO),
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                escrowed_balance: Coin::new(ONE_MICRON.into(), DENOM_UOSMO),
                ..DcaPlusConfig::default()
            }),
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

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

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

    let vault = setup_new_vault(
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

    let vault = setup_new_vault(
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
fn for_inactive_vault_with_active_dca_plus_should_create_a_new_trigger() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                ..Default::default()
            }),
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
fn for_inactive_vault_with_finished_dca_plus_should_not_create_a_new_trigger() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                ..Default::default()
            }),
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

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    assert!(response.messages.contains(&SubMsg {
        id: AFTER_FIN_SWAP_REPLY_ID,
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
                token_out_denom: vault.get_receive_denom(),
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

    let vault = setup_new_vault(
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
        id: AFTER_FIN_SWAP_REPLY_ID,
        msg: MsgSwapExactAmountIn {
            sender: env.contract.address.to_string(),
            token_in: Some(vault.balance.clone().into()),
            token_out_min_amount: Uint128::one().to_string(),
            routes: vec![SwapAmountInRoute {
                pool_id: 3,
                token_out_denom: vault.get_receive_denom(),
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

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            slippage_tolerance: Some(Decimal::percent(1)),
            ..Vault::default()
        },
    );

    let belief_price = Decimal::one();

    let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let token_out_min_amount = get_swap_amount(&deps.as_ref(), &env, &vault)
        .unwrap()
        .amount
        * (Decimal::one() / belief_price)
        * (Decimal::one()
            - Decimal::from_str(OSMOSIS_SWAP_FEE_RATE).unwrap()
            - vault.slippage_tolerance.unwrap());

    assert!(response.messages.contains(&SubMsg {
        id: AFTER_FIN_SWAP_REPLY_ID,
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
                token_out_denom: vault.get_receive_denom(),
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

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            swap_amount: ONE,
            minimum_receive_amount: Some(ONE + ONE),
            ..Vault::default()
        },
    );

    execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(&Event {
        id: 2,
        resource_id: vault.id,
        timestamp: env.block.time,
        block_height: env.block.height,
        data: EventData::DcaVaultExecutionSkipped {
            reason: ExecutionSkippedReason::PriceThresholdExceeded {
                price: Decimal::one()
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

    let vault = setup_new_vault(
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

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            swap_amount: ONE,
            minimum_receive_amount: Some(ONE),
            ..Vault::default()
        },
    );

    let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    assert!(response.messages.contains(&SubMsg {
        id: AFTER_FIN_SWAP_REPLY_ID,
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
                token_out_denom: vault.get_receive_denom(),
            }],
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Always,
    }))
}
