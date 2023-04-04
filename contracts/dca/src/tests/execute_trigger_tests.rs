use super::helpers::{instantiate_contract, setup_vault};

use crate::constants::{ONE, ONE_DECIMAL, TEN, TWO_MICRONS};
use crate::contract::AFTER_FIN_SWAP_REPLY_ID;
use crate::handlers::execute_trigger::execute_trigger_handler;
use crate::handlers::get_events_by_resource_id::get_events_by_resource_id;

use crate::state::triggers::delete_trigger;
use crate::state::vaults::get_vault;
use crate::tests::helpers::setup_vault_2;
use crate::tests::mocks::{calc_mock_dependencies, ADMIN, DENOM_STAKE, DENOM_UOSMO};
use crate::types::dca_plus_config::DcaPlusConfig;
use crate::types::vault::Vault;
use base::events::event::{Event, EventData, ExecutionSkippedReason};

use base::vaults::vault::VaultStatus;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{to_binary, Coin, Decimal, QueryRequest, ReplyOn, SubMsg, Timestamp, Uint128};

use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceResponse;
use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    EstimateSwapExactAmountInResponse, MsgSwapExactAmountIn, SwapAmountInRoute,
};

#[test]
fn when_vault_is_cancelled_should_fail() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        TEN,
        ONE,
        VaultStatus::Cancelled,
        None,
        false,
    );

    let err = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: vault with id 1 is cancelled, and is not available for execution"
    );
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
        TEN,
        ONE,
        VaultStatus::Inactive,
        None,
        false,
    );

    delete_trigger(deps.as_mut().storage, vault.id).unwrap();

    let err = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: vault with id 1 has no trigger attached, and is not available for execution"
    );
}

#[test]
fn when_trigger_is_not_ready_to_fire_should_fail() {
    let mut deps = calc_mock_dependencies();
    let mut env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        TEN,
        ONE,
        VaultStatus::Inactive,
        None,
        false,
    );

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
        TEN,
        ONE,
        VaultStatus::Scheduled,
        None,
        false,
    );

    deps.querier.update_stargate(|_| {
        to_binary(&QuerySpotPriceResponse {
            spot_price: "1.0".to_string(),
        })
        .unwrap()
    });

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
        TEN,
        ONE,
        VaultStatus::Scheduled,
        None,
        false,
    );

    deps.querier.update_stargate(|_| {
        to_binary(&QuerySpotPriceResponse {
            spot_price: "1.0".to_string(),
        })
        .unwrap()
    });

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

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        TEN,
        ONE,
        VaultStatus::Scheduled,
        None,
        false,
    );

    deps.querier.update_stargate(|_| {
        to_binary(&QuerySpotPriceResponse {
            spot_price: "1.0".to_string(),
        })
        .unwrap()
    });

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
fn with_dca_plus_should_simulate_swap() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        TEN,
        ONE,
        VaultStatus::Active,
        None,
        true,
    );

    deps.querier.update_stargate(|query| match query {
        QueryRequest::Stargate { path, .. } => match path.as_str() {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: ONE_DECIMAL.to_string(),
            })
            .unwrap(),
            "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                to_binary(&EstimateSwapExactAmountInResponse {
                    token_out_amount: ONE.to_string(),
                })
                .unwrap()
            }
            _ => panic!("unexpected query"),
        },
        _ => panic!("unexpected query"),
    });

    execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

    assert_eq!(
        updated_vault.dca_plus_config.unwrap(),
        DcaPlusConfig {
            standard_dca_swapped_amount: Coin::new(
                vault.swap_amount.into(),
                vault.get_swap_denom()
            ),
            standard_dca_received_amount: Coin::new(ONE.into(), vault.get_receive_denom()),
            ..vault.dca_plus_config.unwrap()
        }
    );
}

#[test]
fn with_dca_plus_and_exceeded_slippage_tolerance_should_skip_swap() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault_2(
        deps.as_mut(),
        env.clone(),
        Vault {
            slippage_tolerance: Some(Decimal::percent(1)),
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

    deps.querier.update_stargate(|query| match query {
        QueryRequest::Stargate { path, .. } => match path.as_str() {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: ONE_DECIMAL.to_string(),
            })
            .unwrap(),
            "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                to_binary(&EstimateSwapExactAmountInResponse {
                    token_out_amount: (ONE / TWO_MICRONS).to_string(),
                })
                .unwrap()
            }
            _ => panic!("unexpected query"),
        },
        _ => panic!("unexpected query"),
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
fn with_dca_plus_and_exceeded_slippage_tolerance_should_publish_execution_skipped_event() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault_2(
        deps.as_mut(),
        env.clone(),
        Vault {
            slippage_tolerance: Some(Decimal::percent(1)),
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

    deps.querier.update_stargate(|query| match query {
        QueryRequest::Stargate { path, .. } => match path.as_str() {
            "/osmosis.gamm.v2.Query/SpotPrice" => to_binary(&QuerySpotPriceResponse {
                spot_price: ONE_DECIMAL.to_string(),
            })
            .unwrap(),
            "/osmosis.poolmanager.v1beta1.Query/EstimateSwapExactAmountIn" => {
                to_binary(&EstimateSwapExactAmountInResponse {
                    token_out_amount: (ONE / TWO_MICRONS).to_string(),
                })
                .unwrap()
            }
            _ => panic!("unexpected query"),
        },
        _ => panic!("unexpected query"),
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
fn should_create_swap_message() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        TEN,
        ONE,
        VaultStatus::Active,
        None,
        false,
    );

    deps.querier.update_stargate(|_| {
        to_binary(&QuerySpotPriceResponse {
            spot_price: "1.0".to_string(),
        })
        .unwrap()
    });

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
                pool_id: 0,
                token_out_denom: vault.get_receive_denom(),
            }],
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Always,
    }))
}

#[test]
fn should_create_a_new_time_trigger() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        TEN,
        ONE,
        VaultStatus::Active,
        None,
        false,
    );

    deps.querier.update_stargate(|_| {
        to_binary(&QuerySpotPriceResponse {
            spot_price: "1.0".to_string(),
        })
        .unwrap()
    });

    execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

    assert_eq!(vault.trigger.unwrap().as_time().unwrap(), &env.block.time);
    assert_eq!(
        updated_vault.trigger.unwrap().as_time().unwrap(),
        &Timestamp::from_seconds(env.block.time.plus_seconds(24 * 60 * 60).seconds())
    );
}

// #[test]
// fn for_ready_time_trigger_should_update_vault_stats() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             None,
//         );

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault {
//                 vault_id: mock.vault_ids.get("time").unwrap().to_owned(),
//             },
//         )
//         .unwrap();

//     assert_eq!(vault_response.vault.swapped_amount.amount, swap_amount);
//     assert_eq!(vault_response.vault.swapped_amount.denom, DENOM_UOSMO);
//     assert_eq!(
//         vault_response.vault.received_amount.amount,
//         swap_amount - checked_mul(swap_amount, mock.fee_percent).ok().unwrap()
//     );
//     assert_eq!(vault_response.vault.received_amount.denom, DENOM_STAKE);
// }

// #[test]
// fn for_ready_time_trigger_should_create_events() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             None,
//         );

//     let swap_amount_after_fee =
//         swap_amount - checked_mul(swap_amount, mock.fee_percent).ok().unwrap();

//     let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     assert_events_published(
//         &mock,
//         vault_id,
//         &[
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionTriggered {
//                     base_denom: DENOM_STAKE.to_string(),
//                     quote_denom: DENOM_UOSMO.to_string(),
//                     asset_price: Decimal::from_str("1.0").unwrap(),
//                 },
//             )
//             .build(3),
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionCompleted {
//                     sent: Coin::new(swap_amount.into(), DENOM_UOSMO),
//                     received: Coin::new(swap_amount.into(), DENOM_STAKE),
//                     fee: Coin::new((swap_amount - swap_amount_after_fee).into(), DENOM_STAKE),
//                 },
//             )
//             .build(4),
//         ],
//     );
// }

// #[test]
// fn for_ready_time_trigger_should_delete_current_time_trigger() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             None,
//         );

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     let response = mock
//         .app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap_err();

//     assert_eq!(
//         response.root_cause().to_string(),
//         "Error: trigger execution time has not yet elapsed"
//     )
// }

// #[test]
// fn for_ready_time_trigger_should_create_new_time_trigger() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_pass_slippage_tolerance())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             None,
//         );

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     mock.elapse_time(3700);

//     let get_time_trigger_ids_response: TriggerIdsResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address.clone(),
//             &QueryMsg::GetTimeTriggerIds { limit: None },
//         )
//         .unwrap();

//     assert_eq!(get_time_trigger_ids_response.trigger_ids.len(), 1);
// }

// #[test]
// fn for_ready_time_trigger_should_distribute_to_multiple_destinations_properly() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut destinations = vec![];

//     for i in 0..5 {
//         destinations.push(Destination {
//             address: Addr::unchecked(format!("{}-{:?}", USER, i)),
//             allocation: Decimal::percent(20),
//             action: PostExecutionAction::Send,
//         });
//     }

//     let mut mock = MockApp::new(fin_contract_filled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             Some(destinations.clone()),
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             None,
//         );

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     let swap_amount_after_fee =
//         swap_amount - checked_mul(swap_amount, mock.fee_percent).ok().unwrap();

//     assert_address_balances(
//         &mock,
//         &destinations
//             .iter()
//             .map(|destination| {
//                 (
//                     &destination.address,
//                     DENOM_STAKE,
//                     swap_amount_after_fee * destination.allocation,
//                 )
//             })
//             .collect::<Vec<_>>(),
//     );
// }

// #[test]
// fn for_ready_time_trigger_within_price_threshold_should_succeed() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             Some(Uint128::new(99)),
//             None,
//         );

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

//     let swap_amount_after_fee =
//         swap_amount - checked_mul(swap_amount, mock.fee_percent).ok().unwrap();

//     assert_events_published(
//         &mock,
//         vault_id,
//         &[
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionTriggered {
//                     base_denom: DENOM_STAKE.to_string(),
//                     quote_denom: DENOM_UOSMO.to_string(),
//                     asset_price: Decimal::from_str("1.0").unwrap(),
//                 },
//             )
//             .build(3),
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionCompleted {
//                     sent: Coin::new(swap_amount.into(), DENOM_UOSMO),
//                     received: Coin::new(swap_amount.into(), DENOM_STAKE),
//                     fee: Coin::new((swap_amount - swap_amount_after_fee).into(), DENOM_STAKE),
//                 },
//             )
//             .build(4),
//         ],
//     );
// }

// #[test]
// fn for_ready_time_trigger_with_dca_plus_should_withhold_escrow() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             Some(true),
//         );

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::UpdateSwapAdjustments {
//                 position_type: PositionType::Enter,
//                 adjustments: vec![
//                     (30, Decimal::from_str("1.0").unwrap()),
//                     (35, Decimal::from_str("1.0").unwrap()),
//                     (40, Decimal::from_str("1.0").unwrap()),
//                     (45, Decimal::from_str("1.0").unwrap()),
//                     (50, Decimal::from_str("1.0").unwrap()),
//                     (55, Decimal::from_str("1.0").unwrap()),
//                     (60, Decimal::from_str("1.0").unwrap()),
//                     (70, Decimal::from_str("1.0").unwrap()),
//                     (80, Decimal::from_str("1.0").unwrap()),
//                     (90, Decimal::from_str("1.0").unwrap()),
//                 ],
//             },
//             &[],
//         )
//         .unwrap();

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault {
//                 vault_id: mock.vault_ids.get("time").unwrap().to_owned(),
//             },
//         )
//         .unwrap();

//     let escrow_level = vault_response.vault.dca_plus_config.unwrap().escrow_level;

//     let receive_amount_after_escrow =
//         swap_amount - checked_mul(swap_amount, escrow_level).ok().unwrap();

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, Uint128::new(0)),
//             (&user_address, DENOM_STAKE, receive_amount_after_escrow),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit - swap_amount,
//             ),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_STAKE,
//                 ONE_THOUSAND + (swap_amount - receive_amount_after_escrow),
//             ),
//             (
//                 &mock.fin_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + swap_amount,
//             ),
//             (
//                 &mock.fin_contract_address,
//                 DENOM_STAKE,
//                 ONE_THOUSAND - swap_amount,
//             ),
//         ],
//     );
//     assert_eq!(
//         escrow_level * swap_amount,
//         swap_amount - receive_amount_after_escrow
//     );
//     assert!(escrow_level > Decimal::zero());
// }

// #[test]
// fn for_ready_time_trigger_with_dca_plus_should_adjust_swap_amount() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             Some(true),
//         );

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::UpdateSwapAdjustments {
//                 position_type: PositionType::Enter,
//                 adjustments: vec![
//                     (30, Decimal::from_str("1.3").unwrap()),
//                     (35, Decimal::from_str("1.3").unwrap()),
//                     (40, Decimal::from_str("1.3").unwrap()),
//                     (45, Decimal::from_str("1.3").unwrap()),
//                     (50, Decimal::from_str("1.3").unwrap()),
//                     (55, Decimal::from_str("1.3").unwrap()),
//                     (60, Decimal::from_str("1.3").unwrap()),
//                     (70, Decimal::from_str("1.3").unwrap()),
//                     (80, Decimal::from_str("1.3").unwrap()),
//                     (90, Decimal::from_str("1.3").unwrap()),
//                 ],
//             },
//             &[],
//         )
//         .unwrap();

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault {
//                 vault_id: mock.vault_ids.get("time").unwrap().to_owned(),
//             },
//         )
//         .unwrap();

//     assert_eq!(
//         vault_response.vault.swapped_amount.amount,
//         swap_amount * Decimal::from_str("1.3").unwrap()
//     );
// }

// #[test]
// fn for_ready_time_trigger_for_fin_buy_less_than_minimum_receive_amount_should_skip_execution() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             Some(ONE + Uint128::one()),
//             None,
//         );

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
//             (&user_address, DENOM_STAKE, Uint128::new(0)),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit,
//             ),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
//         ],
//     );

//     let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
//             (&user_address, DENOM_STAKE, Uint128::new(0)),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit,
//             ),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
//         ],
//     );

//     assert_events_published(
//         &mock,
//         vault_id,
//         &[
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionTriggered {
//                     base_denom: DENOM_STAKE.to_string(),
//                     quote_denom: DENOM_UOSMO.to_string(),
//                     asset_price: Decimal::from_str("1.0").unwrap(),
//                 },
//             )
//             .build(3),
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionSkipped {
//                     reason: base::events::event::ExecutionSkippedReason::PriceThresholdExceeded {
//                         price: Decimal::from_str("1.0").unwrap(),
//                     },
//                 },
//             )
//             .build(4),
//         ],
//     );

//     assert_vault_balance(
//         &mock,
//         &mock.dca_contract_address,
//         user_address,
//         Uint128::new(1),
//         TEN,
//     );
// }

// #[test]
// fn for_ready_time_trigger_for_fin_sell_less_than_minimum_receive_amount_should_skip_execution() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_STAKE)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_STAKE),
//             swap_amount,
//             "time",
//             Some(ONE + Uint128::one()),
//             None,
//         );

//     let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     assert_events_published(
//         &mock,
//         vault_id,
//         &[
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionTriggered {
//                     base_denom: DENOM_STAKE.to_string(),
//                     quote_denom: DENOM_UOSMO.to_string(),
//                     asset_price: Decimal::from_str("1.0").unwrap(),
//                 },
//             )
//             .build(3),
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionSkipped {
//                     reason: base::events::event::ExecutionSkippedReason::PriceThresholdExceeded {
//                         price: Decimal::from_str("1.0").unwrap(),
//                     },
//                 },
//             )
//             .build(4),
//         ],
//     );

//     assert_vault_balance(
//         &mock,
//         &mock.dca_contract_address,
//         user_address,
//         Uint128::new(1),
//         TEN,
//     );
// }

// #[test]
// fn for_ready_time_trigger_for_less_than_minimum_receive_amount_should_set_new_time_trigger() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;

//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             Some(ONE + Uint128::one()),
//             None,
//         );

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     let get_time_trigger_ids_response: TriggerIdsResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address.clone(),
//             &QueryMsg::GetTimeTriggerIds { limit: None },
//         )
//         .unwrap();

//     assert_eq!(get_time_trigger_ids_response.trigger_ids.len(), 0);

//     mock.elapse_time(3700);

//     let get_time_trigger_ids_response: TriggerIdsResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address.clone(),
//             &QueryMsg::GetTimeTriggerIds { limit: None },
//         )
//         .unwrap();

//     assert_eq!(get_time_trigger_ids_response.trigger_ids.len(), 1);
// }

// #[test]
// fn for_ready_time_trigger_when_slippage_exceeds_limit_should_skip_execution() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_fail_slippage_tolerance())
//         .with_funds_for(&user_address, TEN, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             None,
//         );

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
//             (&user_address, DENOM_STAKE, Uint128::new(0)),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit,
//             ),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
//         ],
//     );

//     mock.elapse_time(10);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap();

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
//             (&user_address, DENOM_STAKE, Uint128::new(0)),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit,
//             ),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
//         ],
//     );

//     assert_vault_balance(
//         &mock,
//         &mock.dca_contract_address,
//         user_address,
//         Uint128::new(1),
//         vault_deposit,
//     );
// }

// #[test]
// fn for_not_ready_time_trigger_should_fail() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
//         .with_funds_for(&user_address, TEN, DENOM_UOSMO)
//         .with_vault_with_time_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             None,
//             None,
//         );

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
//             (&user_address, DENOM_STAKE, Uint128::new(0)),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit,
//             ),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
//         ],
//     );

//     let response = mock
//         .app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::new(1),
//             },
//             &[],
//         )
//         .unwrap_err();

//     assert_eq!(
//         response.root_cause().to_string(),
//         "Error: trigger execution time has not yet elapsed"
//     );

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
//             (&user_address, DENOM_STAKE, Uint128::new(0)),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit,
//             ),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
//         ],
//     );

//     assert_vault_balance(
//         &mock,
//         &mock.dca_contract_address,
//         user_address,
//         Uint128::new(1),
//         TEN,
//     );
// }

// #[test]
// fn until_vault_is_empty_should_update_address_balances() {
//     let user_address = Addr::unchecked(USER);
//     let user_funds = ONE_HUNDRED;
//     let vault_deposit = ONE * Uint128::new(3) / Uint128::new(2);
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_filled_limit_order())
//         .with_funds_for(&user_address, user_funds, DENOM_UOSMO)
//         .with_vault_with_filled_fin_limit_price_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
//             swap_amount,
//             "fin",
//         );

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_funds - vault_deposit),
//             (&user_address, DENOM_STAKE, Uint128::zero()),
//             (
//                 &mock.dca_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit - TWO_MICRONS,
//             ),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (
//                 &mock.fin_contract_address,
//                 DENOM_STAKE,
//                 ONE_THOUSAND + TWO_MICRONS,
//             ),
//         ],
//     );

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault {
//                 vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
//             },
//         )
//         .unwrap();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: vault_response.vault.id,
//             },
//             &[],
//         )
//         .unwrap();

//     mock.elapse_time(3700);

//     let time_triggers: TriggerIdsResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &QueryMsg::GetTimeTriggerIds { limit: None },
//         )
//         .unwrap();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: time_triggers.trigger_ids[0],
//             },
//             &[],
//         )
//         .unwrap();

//     let vault_deposit_minus_limit_order = vault_deposit - TWO_MICRONS;
//     let vault_deposit_after_swap_fee =
//         vault_deposit_minus_limit_order - vault_deposit_minus_limit_order * mock.fee_percent;

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, ONE_HUNDRED - vault_deposit),
//             (
//                 &user_address,
//                 DENOM_STAKE,
//                 vault_deposit_after_swap_fee + TWO_MICRONS,
//             ),
//             (&mock.dca_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (
//                 &mock.fin_contract_address,
//                 DENOM_UOSMO,
//                 ONE_THOUSAND + vault_deposit - TWO_MICRONS,
//             ),
//             (
//                 &mock.fin_contract_address,
//                 DENOM_STAKE,
//                 ONE_THOUSAND - vault_deposit + TWO_MICRONS,
//             ),
//         ],
//     );
// }

// #[test]
// fn until_vault_is_empty_should_update_vault_balance() {
//     let user_address = Addr::unchecked(USER);
//     let user_funds = ONE_HUNDRED;
//     let vault_deposit = ONE * Uint128::new(3) / Uint128::new(2);
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_filled_limit_order())
//         .with_funds_for(&user_address, user_funds, DENOM_UOSMO)
//         .with_vault_with_filled_fin_limit_price_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
//             swap_amount,
//             "fin",
//         );

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault {
//                 vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
//             },
//         )
//         .unwrap();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: vault_response.vault.id,
//             },
//             &[],
//         )
//         .unwrap();

//     mock.elapse_time(3700);

//     let time_triggers: TriggerIdsResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &QueryMsg::GetTimeTriggerIds { limit: None },
//         )
//         .unwrap();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: time_triggers.trigger_ids[0],
//             },
//             &[],
//         )
//         .unwrap();

//     assert_vault_balance(
//         &mock,
//         &mock.dca_contract_address,
//         user_address,
//         vault_response.vault.id,
//         Uint128::new(0),
//     );
// }

// #[test]
// fn until_vault_is_empty_should_update_vault_status() {
//     let user_address = Addr::unchecked(USER);
//     let user_funds = ONE_HUNDRED;
//     let vault_deposit = Uint128::new(100000);
//     let swap_amount = Uint128::new(60000);
//     let mut mock = MockApp::new(fin_contract_filled_limit_order())
//         .with_funds_for(&user_address, user_funds, DENOM_UOSMO)
//         .with_vault_with_filled_fin_limit_price_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
//             swap_amount,
//             "fin",
//         );

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::one(),
//             },
//             &[],
//         )
//         .unwrap();

//     mock.elapse_time(3700);

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: Uint128::one(),
//             },
//             &[],
//         )
//         .unwrap();

//     let vault = mock
//         .app
//         .wrap()
//         .query_wasm_smart::<VaultResponse>(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault {
//                 vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
//             },
//         )
//         .unwrap()
//         .vault;

//     assert_eq!(vault.status, VaultStatus::Inactive);
// }

// #[test]
// fn until_vault_is_empty_should_update_vault_stats() {
//     let user_address = Addr::unchecked(USER);
//     let user_funds = ONE_HUNDRED;
//     let vault_deposit = ONE * Uint128::new(3) / Uint128::new(2);
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_filled_limit_order())
//         .with_funds_for(&user_address, user_funds, DENOM_UOSMO)
//         .with_vault_with_filled_fin_limit_price_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
//             swap_amount,
//             "fin",
//         );

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault {
//                 vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
//             },
//         )
//         .unwrap();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: vault_response.vault.id,
//             },
//             &[],
//         )
//         .unwrap();

//     mock.elapse_time(3700);

//     let time_triggers: TriggerIdsResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &QueryMsg::GetTimeTriggerIds { limit: None },
//         )
//         .unwrap();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: time_triggers.trigger_ids[0],
//             },
//             &[],
//         )
//         .unwrap();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault {
//                 vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
//             },
//         )
//         .unwrap();

//     let vault_deposit_after_limit_order = vault_deposit - TWO_MICRONS;
//     let vault_deposit_after_swap_fees = vault_deposit_after_limit_order
//         - vault_deposit_after_limit_order * mock.fee_percent
//         + TWO_MICRONS;

//     assert_eq!(
//         vault_response.vault.swapped_amount.amount,
//         vault_deposit - TWO_MICRONS
//     );
//     assert_eq!(vault_response.vault.swapped_amount.denom, DENOM_UOSMO);
//     assert_eq!(
//         vault_response.vault.received_amount.amount,
//         vault_deposit_after_swap_fees
//     );
//     assert_eq!(vault_response.vault.received_amount.denom, DENOM_STAKE);
// }

// #[test]
// fn until_vault_is_empty_should_create_events() {
//     let user_address = Addr::unchecked(USER);
//     let user_funds = ONE_HUNDRED;
//     let vault_deposit = ONE * Uint128::new(3) / Uint128::new(2);
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_filled_limit_order())
//         .with_funds_for(&user_address, user_funds, DENOM_UOSMO)
//         .with_vault_with_filled_fin_limit_price_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
//             swap_amount,
//             "fin",
//         );

//     let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: vault_id,
//             },
//             &[],
//         )
//         .unwrap();

//     let initial_block_info = mock.app.block_info();

//     mock.elapse_time(3700);

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault { vault_id },
//         )
//         .unwrap();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: vault_id,
//             },
//             &[],
//         )
//         .unwrap();

//     let remaining_swap_amount = vault_response.vault.balance.amount;

//     assert_events_published(
//         &mock,
//         vault_id,
//         &[
//             EventBuilder::new(
//                 vault_id,
//                 initial_block_info.clone(),
//                 EventData::DcaVaultExecutionTriggered {
//                     base_denom: DENOM_STAKE.to_string(),
//                     quote_denom: DENOM_UOSMO.to_string(),
//                     asset_price: Decimal::from_str("1.0").unwrap(),
//                 },
//             )
//             .build(3),
//             EventBuilder::new(
//                 vault_id,
//                 initial_block_info.clone(),
//                 EventData::DcaVaultExecutionCompleted {
//                     sent: Coin::new(swap_amount.into(), DENOM_UOSMO),
//                     received: Coin::new((swap_amount + TWO_MICRONS).into(), DENOM_STAKE),
//                     fee: Coin::new((swap_amount * mock.fee_percent).into(), DENOM_STAKE),
//                 },
//             )
//             .build(4),
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionTriggered {
//                     base_denom: DENOM_STAKE.to_string(),
//                     quote_denom: DENOM_UOSMO.to_string(),
//                     asset_price: Decimal::from_str("1.0").unwrap(),
//                 },
//             )
//             .build(5),
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultExecutionCompleted {
//                     sent: Coin::new(remaining_swap_amount.into(), DENOM_UOSMO),
//                     received: Coin::new(remaining_swap_amount.into(), DENOM_STAKE),
//                     fee: Coin::new(
//                         (remaining_swap_amount * mock.fee_percent).into(),
//                         DENOM_STAKE,
//                     ),
//                 },
//             )
//             .build(6),
//         ],
//     );
// }

// #[test]
// fn when_contract_is_paused_should_fail() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_filled_limit_order())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_vault_with_filled_fin_limit_price_trigger(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "fin",
//         );

//     let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault { vault_id },
//         )
//         .unwrap();

//     mock.app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::UpdateConfig {
//                 fee_collectors: Some(vec![FeeCollector {
//                     address: ADMIN.to_string(),
//                     allocation: Decimal::from_str("1").unwrap(),
//                 }]),
//                 swap_fee_percent: Some(Decimal::from_str("0.015").unwrap()),
//                 delegation_fee_percent: Some(Decimal::from_str("0.0075").unwrap()),
//                 staking_router_address: None,
//                 page_limit: None,
//                 paused: Some(true),
//                 dca_plus_escrow_level: None,
//             },
//             &[],
//         )
//         .unwrap();

//     let response = mock
//         .app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::ExecuteTrigger {
//                 trigger_id: vault_response.vault.id,
//             },
//             &[],
//         )
//         .unwrap_err();

//     assert_eq!(
//         "Error: contract is paused",
//         response.root_cause().to_string()
//     )
// }

// #[test]
// fn for_fin_buy_vault_with_exceeded_price_ceiling_should_skip_execution() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = ONE;
//     let vault_deposit = Uint128::new(100000);
//     let swap_amount = Uint128::new(60000);

//     let mock = MockApp::new(fin_contract_high_swap_price())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_active_vault(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             Some(swap_amount),
//         );

//     let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault { vault_id },
//         )
//         .unwrap();

//     assert_eq!(vault_response.vault.balance.amount, vault_deposit);
// }

// #[test]
// fn for_fin_buy_vault_with_non_exceeded_price_ceiling_should_execute() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = ONE;
//     let vault_deposit = Uint128::new(100000);
//     let swap_amount = Uint128::new(60000);

//     let mock = MockApp::new(fin_contract_low_swap_price())
//         .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
//         .with_active_vault(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_UOSMO),
//             swap_amount,
//             "time",
//             Some(swap_amount),
//         );

//     let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault { vault_id },
//         )
//         .unwrap();

//     assert_eq!(
//         vault_response.vault.balance.amount,
//         vault_deposit - swap_amount
//     );
// }

// #[test]
// fn for_fin_sell_vault_with_exceeded_price_floor_should_skip_execution() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = ONE;
//     let vault_deposit = Uint128::new(100000);
//     let swap_amount = Uint128::new(60000);

//     let mock = MockApp::new(fin_contract_low_swap_price())
//         .with_funds_for(&user_address, user_balance, DENOM_STAKE)
//         .with_active_vault(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_STAKE),
//             swap_amount,
//             "time",
//             Some(swap_amount),
//         );

//     let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault { vault_id },
//         )
//         .unwrap();

//     assert_eq!(vault_response.vault.balance.amount, vault_deposit);
// }

// #[test]
// fn for_fin_sell_vault_with_non_exceeded_price_floor_should_execute() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = ONE;
//     let vault_deposit = Uint128::new(100000);
//     let swap_amount = Uint128::new(60000);

//     let mock = MockApp::new(fin_contract_high_swap_price())
//         .with_funds_for(&user_address, user_balance, DENOM_STAKE)
//         .with_active_vault(
//             &user_address,
//             None,
//             Coin::new(vault_deposit.into(), DENOM_STAKE),
//             swap_amount,
//             "time",
//             Some(swap_amount),
//         );

//     let vault_id = mock.vault_ids.get("time").unwrap().to_owned();

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(
//             &mock.dca_contract_address,
//             &&QueryMsg::GetVault { vault_id },
//         )
//         .unwrap();

//     assert_eq!(
//         vault_response.vault.balance.amount,
//         vault_deposit - swap_amount
//     );
// }

// #[test]
// fn for_active_vault_creates_new_trigger() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Active,
//         false,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

//     assert_eq!(
//         updated_vault.trigger,
//         Some(TriggerConfiguration::Time {
//             target_time: get_next_target_time(env.block.time, env.block.time, vault.time_interval)
//         })
//     );
// }

// #[test]
// fn for_active_vault_with_dca_plus_updates_standard_performance_data() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Active,
//         true,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_dca_plus_config = get_vault(deps.as_ref().storage, vault.id)
//         .unwrap()
//         .dca_plus_config
//         .unwrap();

//     let price = query_price(
//         deps.as_ref().querier,
//         &env,
//         &vault.pool,
//         &Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
//     )
//     .unwrap();

//     let fee_rate = get_swap_fee_rate(&deps.as_mut(), &vault).unwrap()
//         + get_delegation_fee_rate(&deps.as_mut(), &vault).unwrap()
//         + Decimal::from_str(FIN_TAKER_FEE).unwrap();

//     assert_eq!(
//         updated_dca_plus_config.standard_dca_swapped_amount.amount,
//         vault.swap_amount
//     );
//     assert_eq!(
//         updated_dca_plus_config.standard_dca_received_amount.amount,
//         vault.swap_amount * (Decimal::one() / price) * (Decimal::one() - fee_rate)
//     );
// }

// #[test]
// fn for_active_vault_with_dca_plus_publishes_execution_simulated_event() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Active,
//         true,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
//         .unwrap()
//         .events;

//     let dca_plus_config = get_vault(deps.as_ref().storage, vault.id)
//         .unwrap()
//         .dca_plus_config
//         .unwrap();

//     let config = get_config(deps.as_ref().storage).unwrap();

//     let fee = (config.swap_fee_percent
//         + config.delegation_fee_percent
//         + Decimal::from_str(FIN_TAKER_FEE).unwrap())
//         * dca_plus_config.standard_dca_received_amount.amount;

//     assert!(events.contains(&Event {
//         id: 2,
//         timestamp: env.block.time,
//         block_height: env.block.height,
//         resource_id: vault.id,
//         data: EventData::SimulatedDcaVaultExecutionCompleted {
//             sent: dca_plus_config.standard_dca_swapped_amount,
//             received: dca_plus_config.standard_dca_received_amount,
//             fee: Coin::new(fee.into(), vault.get_receive_denom())
//         },
//     }));
// }

// #[test]
// fn for_active_dca_plus_vault_with_finished_standard_dca_does_not_update_stats() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let mut vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Active,
//         true,
//     );

//     vault.dca_plus_config = vault
//         .dca_plus_config
//         .clone()
//         .map(|dca_plus_config| DcaPlusConfig {
//             standard_dca_swapped_amount: Coin::new(TEN.into(), vault.get_swap_denom()),
//             standard_dca_received_amount: Coin::new(TEN.into(), vault.get_receive_denom()),
//             ..dca_plus_config
//         });

//     update_vault(deps.as_mut().storage, &vault).unwrap();

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

//     assert_eq!(
//         DcaPlusConfig {
//             standard_dca_swapped_amount: Coin::new(TEN.into(), vault.get_swap_denom()),
//             standard_dca_received_amount: Coin::new(TEN.into(), vault.get_receive_denom()),
//             ..vault.dca_plus_config.unwrap()
//         },
//         updated_vault.dca_plus_config.unwrap()
//     );
// }

// #[test]
// fn for_active_vault_with_slippage_exceeded_publishes_standard_dca_execution_skipped_event() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &ONE_MICRON, &ONE_MICRON);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Active,
//         true,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
//         .unwrap()
//         .events;

//     assert!(events.contains(&Event {
//         id: 2,
//         timestamp: env.block.time,
//         block_height: env.block.height,
//         resource_id: vault.id,
//         data: EventData::SimulatedDcaVaultExecutionSkipped {
//             reason: ExecutionSkippedReason::SlippageToleranceExceeded,
//         },
//     }));
// }

// #[test]
// fn for_scheduled_vault_updates_status_to_active() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Scheduled,
//         false,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

//     assert_eq!(updated_vault.status, VaultStatus::Active);
// }

// #[test]
// fn for_scheduled_vault_creates_new_trigger() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Scheduled,
//         false,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

//     assert_eq!(
//         updated_vault.trigger,
//         Some(TriggerConfiguration::Time {
//             target_time: get_next_target_time(env.block.time, env.block.time, vault.time_interval)
//         })
//     );
// }

// #[test]
// fn for_inactive_vault_does_not_create_a_new_trigger() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Inactive,
//         false,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

//     assert_eq!(updated_vault.trigger, None);
// }

// #[test]
// fn for_inactive_vault_with_dca_plus_creates_new_trigger() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Inactive,
//         true,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

//     assert_eq!(
//         updated_vault.trigger,
//         Some(TriggerConfiguration::Time {
//             target_time: get_next_target_time(env.block.time, env.block.time, vault.time_interval)
//         })
//     );
// }

// #[test]
// fn for_inactive_vault_with_dca_plus_and_finished_standard_dca_does_not_create_new_trigger() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         ONE,
//         ONE,
//         VaultStatus::Inactive,
//         true,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

//     assert_eq!(updated_vault.trigger, None);
// }

// #[test]
// fn for_inactive_vault_with_dca_plus_updates_standard_performance_data() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Inactive,
//         true,
//     );

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     let updated_dca_plus_config = get_vault(deps.as_ref().storage, vault.id)
//         .unwrap()
//         .dca_plus_config
//         .unwrap();

//     let price = query_price(
//         deps.as_ref().querier,
//         &env,
//         &vault.pool,
//         &Coin::new(vault.swap_amount.into(), vault.get_swap_denom()),
//     )
//     .unwrap();

//     let fee_rate = get_swap_fee_rate(&deps.as_mut(), &vault).unwrap()
//         + get_delegation_fee_rate(&deps.as_mut(), &vault).unwrap()
//         + Decimal::from_str(FIN_TAKER_FEE).unwrap();

//     assert_eq!(
//         updated_dca_plus_config.standard_dca_swapped_amount.amount,
//         vault.swap_amount
//     );
//     assert_eq!(
//         updated_dca_plus_config.standard_dca_received_amount.amount,
//         vault.swap_amount * (Decimal::one() / price) * (Decimal::one() - fee_rate)
//     );
// }

// #[test]
// fn for_inactive_dca_plus_vault_with_finished_standard_dca_disburses_escrow() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     let vault = setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         ONE,
//         ONE,
//         VaultStatus::Inactive,
//         true,
//     );

//     let response = execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap();

//     assert!(response
//         .messages
//         .contains(&SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: env.contract.address.to_string(),
//             msg: to_binary(&ExecuteMsg::DisburseEscrow { vault_id: vault.id }).unwrap(),
//             funds: vec![],
//         }))))
// }

// #[test]
// fn for_cancelled_vault_deletes_trigger() {
//     let mut deps = mock_dependencies();
//     let env = mock_env();
//     let info = mock_info(ADMIN, &[]);

//     instantiate_contract(deps.as_mut(), env.clone(), info);
//     // set_fin_price(&mut deps, &ONE_DECIMAL, &TEN, &TEN_MICRONS);

//     setup_vault(
//         deps.as_mut(),
//         env.clone(),
//         TEN,
//         ONE,
//         VaultStatus::Cancelled,
//         true,
//     );

//     let vault = get_vault(deps.as_ref().storage, Uint128::one()).unwrap();

//     execute_trigger_handler(deps.as_mut(), env.clone(), vault.id).unwrap_err();

//     let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

//     assert_eq!(
//         vault.trigger,
//         Some(TriggerConfiguration::Time {
//             target_time: env.block.time
//         })
//     );
//     assert_eq!(updated_vault.trigger, None);
// }
