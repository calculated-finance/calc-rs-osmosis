use crate::handlers::create_pool::create_pool;
use crate::handlers::create_vault::create_vault;
use crate::handlers::get_events_by_resource_id::get_events_by_resource_id;
use crate::handlers::get_vault::get_vault;
use crate::msg::ExecuteMsg;
use crate::state::config::{get_config, update_config, Config};
use crate::tests::helpers::instantiate_contract;
use crate::tests::mocks::{ADMIN, DENOM_STAKE, DENOM_UOSMO, USER};
use crate::types::dca_plus_config::DcaPlusConfig;
use crate::types::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::pool::Pool;
use base::triggers::trigger::{TimeInterval, TriggerConfiguration};
use base::vaults::vault::{Destination, PostExecutionAction, VaultStatus};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, SubMsg, Timestamp, Uint128, WasmMsg,
};

#[test]
fn with_no_assets_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = create_vault(
        deps.as_mut(),
        env,
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(10000),
        TimeInterval::Daily,
        None,
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: received 0 denoms but required exactly 1"
    );
}

#[test]
fn with_multiple_assets_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(
        USER,
        &[Coin::new(10000, DENOM_UOSMO), Coin::new(10000, DENOM_STAKE)],
    );

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = create_vault(
        deps.as_mut(),
        env,
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(10000),
        TimeInterval::Daily,
        None,
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: received 2 denoms but required exactly 1"
    );
}

#[test]
fn with_non_existent_pool_id_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = create_vault(
        deps.as_mut(),
        env,
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        None,
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(err.to_string(), "base::pool::Pool not found");
}

#[test]
fn with_destination_allocations_less_than_100_percent_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = create_vault(
        deps.as_mut(),
        env,
        &info,
        info.sender.clone(),
        None,
        vec![Destination {
            address: Addr::unchecked("destination"),
            allocation: Decimal::percent(50),
            action: PostExecutionAction::Send,
        }],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        None,
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: destination allocations must add up to 1"
    );
}

#[test]
fn with_destination_allocation_equal_to_zero_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = create_vault(
        deps.as_mut(),
        env,
        &info,
        info.sender.clone(),
        None,
        vec![
            Destination {
                address: Addr::unchecked("destination-all"),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send,
            },
            Destination {
                address: Addr::unchecked("destination-empty"),
                allocation: Decimal::percent(0),
                action: PostExecutionAction::Send,
            },
        ],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        None,
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: all destination allocations must be greater than 0"
    );
}

#[test]
fn with_more_than_10_destination_allocations_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = create_vault(
        deps.as_mut(),
        env,
        &info,
        info.sender.clone(),
        None,
        (0..20)
            .into_iter()
            .map(|i| Destination {
                address: Addr::unchecked(format!("destination-{}", i)),
                allocation: Decimal::percent(5),
                action: PostExecutionAction::Send,
            })
            .collect(),
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        None,
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: no more than 10 destinations can be provided"
    );
}

#[test]
fn with_swap_amount_less_than_50000_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = create_vault(
        deps.as_mut(),
        env,
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(10000),
        TimeInterval::Daily,
        None,
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: swap amount must be greater than 50000"
    );
}

#[test]
fn when_contract_is_paused_should_fail() {
    let mut deps = mock_dependencies();
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

    let err = create_vault(
        deps.as_mut(),
        env,
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        None,
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(err.to_string(), "Error: contract is paused")
}

#[test]
fn with_time_trigger_with_target_time_in_the_past_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &[Coin::new(10000, DENOM_STAKE)]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        Some(env.block.time.minus_seconds(10).seconds().into()),
        None,
        Some(false),
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: target_start_time_utc_seconds must be some time in the future"
    );
}

#[test]
fn should_create_vault() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        DENOM_STAKE.to_string(),
        DENOM_UOSMO.to_string(),
    )
    .unwrap();

    let swap_amount = Uint128::new(100000);
    info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

    create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        swap_amount,
        TimeInterval::Daily,
        Some(env.block.time.plus_seconds(10).seconds().into()),
        None,
        Some(false),
    )
    .unwrap();

    let vault = get_vault(deps.as_ref(), Uint128::one()).unwrap().vault;

    assert_eq!(
        vault,
        Vault {
            minimum_receive_amount: None,
            label: None,
            id: Uint128::new(1),
            owner: info.sender,
            destinations: vec![Destination {
                address: Addr::unchecked(USER),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            created_at: env.block.time,
            status: VaultStatus::Scheduled,
            time_interval: TimeInterval::Daily,
            balance: info.funds[0].clone(),
            slippage_tolerance: None,
            swap_amount,
            pool: Pool {
                pool_id: 0,
                base_denom: DENOM_STAKE.to_string(),
                quote_denom: DENOM_UOSMO.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_STAKE.to_string()),
            received_amount: Coin::new(0, DENOM_UOSMO.to_string()),
            trigger: Some(TriggerConfiguration::Time {
                target_time: Timestamp::from_seconds(env.block.time.plus_seconds(10).seconds()),
            }),
            dca_plus_config: None,
        }
    );
}

#[test]
fn should_publish_deposit_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        DENOM_STAKE.to_string(),
        DENOM_UOSMO.to_string(),
    )
    .unwrap();

    info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

    create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        Some(env.block.time.plus_seconds(10).seconds().into()),
        None,
        Some(false),
    )
    .unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), Uint128::one(), None, None)
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
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        DENOM_STAKE.to_string(),
        DENOM_UOSMO.to_string(),
    )
    .unwrap();

    let owner = Addr::unchecked(USER);
    info = mock_info(ADMIN, &[Coin::new(100000, DENOM_STAKE)]);

    create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        owner,
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        Some(env.block.time.plus_seconds(10).seconds().into()),
        None,
        Some(false),
    )
    .unwrap();

    let vault = get_vault(deps.as_ref(), Uint128::one()).unwrap().vault;

    assert_eq!(vault.owner, Addr::unchecked(USER));
}

#[test]
fn with_multiple_destinations_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        DENOM_STAKE.to_string(),
        DENOM_UOSMO.to_string(),
    )
    .unwrap();

    info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

    let destinations = vec![
        Destination {
            address: Addr::unchecked("dest-1"),
            allocation: Decimal::percent(50),
            action: PostExecutionAction::Send,
        },
        Destination {
            address: Addr::unchecked("dest-2"),
            allocation: Decimal::percent(50),
            action: PostExecutionAction::ZDelegate,
        },
    ];

    create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        info.sender.clone(),
        None,
        destinations.clone(),
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        Some(env.block.time.plus_seconds(10).seconds().into()),
        None,
        Some(false),
    )
    .unwrap();

    let vault = get_vault(deps.as_ref(), Uint128::one()).unwrap().vault;

    assert_eq!(vault.destinations, destinations);
}

#[test]
fn with_insufficient_funds_should_create_inactive_vault() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        DENOM_STAKE.to_string(),
        DENOM_UOSMO.to_string(),
    )
    .unwrap();

    info = mock_info(USER, &[Coin::new(1, DENOM_STAKE)]);

    create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        Some(env.block.time.plus_seconds(10).seconds().into()),
        None,
        Some(false),
    )
    .unwrap();

    let vault = get_vault(deps.as_ref(), Uint128::one()).unwrap().vault;

    assert_eq!(vault.status, VaultStatus::Inactive);
}

#[test]
fn with_use_dca_plus_true_should_create_dca_plus_config() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        DENOM_STAKE.to_string(),
        DENOM_UOSMO.to_string(),
    )
    .unwrap();

    info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

    create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        Some(env.block.time.plus_seconds(10).seconds().into()),
        None,
        Some(true),
    )
    .unwrap();

    let config = get_config(deps.as_ref().storage).unwrap();
    let vault = get_vault(deps.as_ref(), Uint128::one()).unwrap().vault;

    assert_eq!(
        vault.dca_plus_config,
        Some(DcaPlusConfig {
            escrow_level: config.dca_plus_escrow_level,
            model_id: 30,
            total_deposit: info.funds[0].clone(),
            standard_dca_swapped_amount: Coin::new(0, vault.balance.denom),
            standard_dca_received_amount: Coin::new(0, DENOM_UOSMO),
            escrowed_balance: Coin::new(0, DENOM_UOSMO)
        })
    );
}

#[test]
fn with_large_deposit_should_select_longer_duration_model() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        DENOM_STAKE.to_string(),
        DENOM_UOSMO.to_string(),
    )
    .unwrap();

    info = mock_info(USER, &[Coin::new(1000000000, DENOM_STAKE)]);

    create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        Some(env.block.time.plus_seconds(10).seconds().into()),
        None,
        Some(true),
    )
    .unwrap();

    let vault = get_vault(deps.as_ref(), Uint128::one()).unwrap().vault;

    assert_eq!(vault.dca_plus_config.unwrap().model_id, 90);
}

#[test]
fn with_no_target_time_should_execute_vault() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        DENOM_STAKE.to_string(),
        DENOM_UOSMO.to_string(),
    )
    .unwrap();

    info = mock_info(USER, &[Coin::new(100000, DENOM_STAKE)]);

    let response = create_vault(
        deps.as_mut(),
        env.clone(),
        &info,
        info.sender.clone(),
        None,
        vec![],
        0,
        None,
        None,
        None,
        Uint128::new(100000),
        TimeInterval::Daily,
        None,
        None,
        Some(true),
    )
    .unwrap();

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::ExecuteTrigger {
                trigger_id: Uint128::one()
            })
            .unwrap()
        }))));
}

// #[test]
// fn with_immediate_time_trigger_should_update_vault_balance() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_pass_slippage_tolerance()).with_funds_for(
//         &user_address,
//         user_balance,
//         DENOM_UOSMO,
//     );

//     mock.app
//         .execute_contract(
//             Addr::unchecked(USER),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::CreateVault {
//                 owner: None,
//                 minimum_receive_amount: None,
//                 label: Some("label".to_string()),
//                 destinations: None,
//                 pool_id: 0,
//                 position_type: None,
//                 slippage_tolerance: None,
//                 swap_amount,
//                 time_interval: TimeInterval::Hourly,
//                 target_start_time_utc_seconds: None,
//                 target_receive_amount: None,
//                 use_dca_plus: None,
//             },
//             &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
//         )
//         .unwrap();

//     assert_vault_balance(
//         &mock,
//         &mock.dca_contract_address,
//         user_address,
//         Uint128::new(1),
//         vault_deposit - swap_amount,
//     );
// }

// #[test]
// fn with_immediate_time_trigger_should_create_active_vault() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_pass_slippage_tolerance()).with_funds_for(
//         &user_address,
//         user_balance,
//         DENOM_UOSMO,
//     );

//     mock.app
//         .execute_contract(
//             Addr::unchecked(USER),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::CreateVault {
//                 owner: None,
//                 minimum_receive_amount: None,
//                 label: Some("label".to_string()),
//                 destinations: None,
//                 pool_id: 0,
//                 position_type: None,
//                 slippage_tolerance: None,
//                 swap_amount,
//                 time_interval: TimeInterval::Hourly,
//                 target_start_time_utc_seconds: None,
//                 target_receive_amount: None,
//                 use_dca_plus: None,
//             },
//             &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
//         )
//         .unwrap();

//     let vault_id = Uint128::new(1);

//     let vault_response: VaultResponse = mock
//         .app
//         .wrap()
//         .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
//         .unwrap();

//     assert_eq!(
//         vault_response.vault,
//         Vault {
//             minimum_receive_amount: None,
//             label: Some("label".to_string()),
//             id: vault_id,
//             owner: user_address.clone(),
//             destinations: vec![Destination {
//                 address: user_address.clone(),
//                 allocation: Decimal::percent(100),
//                 action: PostExecutionAction::Send
//             }],
//             created_at: mock.app.block_info().time,
//             status: VaultStatus::Active,
//             time_interval: TimeInterval::Hourly,
//             balance: Coin::new(
//                 (vault_deposit - swap_amount).into(),
//                 DENOM_UOSMO.to_string()
//             ),
//             slippage_tolerance: None,
//             swap_amount,
//             pool: Pool {
//                 pool_id: 0,
//                 base_denom: DENOM_STAKE.to_string(),
//                 quote_denom: DENOM_UOSMO.to_string(),
//             },
//             started_at: Some(mock.app.block_info().time),
//             swapped_amount: Coin::new(swap_amount.into(), DENOM_UOSMO.to_string()),
//             received_amount: Coin::new(
//                 (swap_amount - checked_mul(swap_amount, mock.fee_percent).ok().unwrap()).into(),
//                 DENOM_STAKE.to_string()
//             ),
//             trigger: Some(TriggerConfiguration::Time {
//                 target_time: mock
//                     .app
//                     .block_info()
//                     .time
//                     .plus_seconds(60 * 60)
//                     .minus_nanos(mock.app.block_info().time.subsec_nanos()),
//             }),
//             dca_plus_config: None,
//         }
//     );
// }

// #[test]
// fn with_immediate_time_trigger_should_publish_events() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_pass_slippage_tolerance()).with_funds_for(
//         &user_address,
//         user_balance,
//         DENOM_UOSMO,
//     );

//     mock.app
//         .execute_contract(
//             Addr::unchecked(USER),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::CreateVault {
//                 owner: None,
//                 minimum_receive_amount: None,
//                 label: Some("label".to_string()),
//                 destinations: None,
//                 pool_id: 0,
//                 position_type: None,
//                 slippage_tolerance: None,
//                 swap_amount,
//                 time_interval: TimeInterval::Hourly,
//                 target_start_time_utc_seconds: None,
//                 target_receive_amount: None,
//                 use_dca_plus: None,
//             },
//             &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
//         )
//         .unwrap();

//     let vault_id = Uint128::new(1);

//     assert_events_published(
//         &mock,
//         vault_id,
//         &[
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultCreated {},
//             )
//             .build(1),
//             EventBuilder::new(
//                 vault_id,
//                 mock.app.block_info(),
//                 EventData::DcaVaultFundsDeposited {
//                     amount: Coin::new(vault_deposit.into(), DENOM_UOSMO),
//                 },
//             )
//             .build(2),
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
//                     fee: Coin::new(
//                         (checked_mul(swap_amount, mock.fee_percent).ok().unwrap()).into(),
//                         DENOM_STAKE,
//                     ),
//                 },
//             )
//             .build(4),
//         ],
//     );
// }

// #[test]
// fn with_immediate_time_trigger_and_slippage_failure_should_update_address_balances() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_fail_slippage_tolerance()).with_funds_for(
//         &user_address,
//         user_balance,
//         DENOM_UOSMO,
//     );

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_balance),
//             (&user_address, DENOM_STAKE, Uint128::new(0)),
//             (&mock.dca_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
//             (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
//         ],
//     );

//     mock.app
//         .execute_contract(
//             Addr::unchecked(USER),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::CreateVault {
//                 owner: None,
//                 minimum_receive_amount: None,
//                 label: Some("label".to_string()),
//                 destinations: None,
//                 pool_id: 0,
//                 position_type: None,
//                 slippage_tolerance: None,
//                 swap_amount,
//                 time_interval: TimeInterval::Hourly,
//                 target_start_time_utc_seconds: None,
//                 target_receive_amount: None,
//                 use_dca_plus: None,
//             },
//             &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
//         )
//         .unwrap();

//     assert_address_balances(
//         &mock,
//         &[
//             (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
//             (&user_address, DENOM_STAKE, Uint128::zero()),
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
// }

// #[test]
// fn with_immediate_time_trigger_and_slippage_failure_should_update_vault_balance() {
//     let user_address = Addr::unchecked(USER);
//     let user_balance = TEN;
//     let vault_deposit = TEN;
//     let swap_amount = ONE;
//     let mut mock = MockApp::new(fin_contract_fail_slippage_tolerance()).with_funds_for(
//         &user_address,
//         user_balance,
//         DENOM_UOSMO,
//     );

//     mock.app
//         .execute_contract(
//             Addr::unchecked(USER),
//             mock.dca_contract_address.clone(),
//             &ExecuteMsg::CreateVault {
//                 owner: None,
//                 minimum_receive_amount: None,
//                 label: Some("label".to_string()),
//                 destinations: None,
//                 pool_id: 0,
//                 position_type: None,
//                 slippage_tolerance: None,
//                 swap_amount,
//                 time_interval: TimeInterval::Hourly,
//                 target_start_time_utc_seconds: None,
//                 target_receive_amount: None,
//                 use_dca_plus: None,
//             },
//             &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
//         )
//         .unwrap();

//     assert_vault_balance(
//         &mock,
//         &mock.dca_contract_address,
//         user_address,
//         Uint128::new(1),
//         vault_deposit,
//     );
// }
