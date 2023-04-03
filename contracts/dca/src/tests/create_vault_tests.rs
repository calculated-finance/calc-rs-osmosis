use super::mocks::{fin_contract_fail_slippage_tolerance, fin_contract_pass_slippage_tolerance};
use crate::constants::{ONE, ONE_THOUSAND, TEN, TWO_MICRONS};
use crate::handlers::create_vault::create_vault;
use crate::msg::{ExecuteMsg, QueryMsg, VaultResponse};
use crate::state::config::{get_config, update_config, Config, FeeCollector};
use crate::tests::helpers::{
    assert_address_balances, assert_events_published, assert_vault_balance, instantiate_contract,
};
use crate::tests::instantiate_tests::VALID_ADDRESS_ONE;
use crate::tests::mocks::{
    fin_contract_unfilled_limit_order, MockApp, ADMIN, DENOM_STAKE, DENOM_UOSMO, USER,
};
use crate::types::dca_plus_config::DcaPlusConfig;
use crate::types::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::helpers::math_helpers::checked_mul;
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::pool::Pool;
use base::triggers::trigger::{TimeInterval, TriggerConfiguration};
use base::vaults::vault::{Destination, PostExecutionAction, VaultStatus};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{Addr, Coin, Decimal, Decimal256, Uint128, Uint64};
use cw_multi_test::Executor;
use std::str::FromStr;

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
        Addr::unchecked(USER),
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
        Addr::unchecked(USER),
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
        Addr::unchecked(USER),
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
        Addr::unchecked(USER),
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
        Addr::unchecked(USER),
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
        Addr::unchecked(USER),
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
        Addr::unchecked(USER),
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
        Addr::unchecked(USER),
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
        Addr::unchecked(USER),
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
fn with_immediate_time_trigger_should_update_address_balances() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_pass_slippage_tolerance()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance),
            (&user_address, DENOM_STAKE, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
        ],
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let fee_amount = checked_mul(swap_amount, mock.fee_percent).ok().unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
            (&user_address, DENOM_STAKE, swap_amount - fee_amount),
            (
                &mock.dca_contract_address,
                DENOM_UOSMO,
                ONE_THOUSAND + user_balance - swap_amount,
            ),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UOSMO,
                ONE_THOUSAND + swap_amount,
            ),
            (
                &mock.fin_contract_address,
                DENOM_STAKE,
                ONE_THOUSAND - swap_amount,
            ),
        ],
    );
}

#[test]
fn with_immediate_time_trigger_should_update_vault_balance() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_pass_slippage_tolerance()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        user_address,
        Uint128::new(1),
        vault_deposit - swap_amount,
    );
}

#[test]
fn with_immediate_time_trigger_should_create_active_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_pass_slippage_tolerance()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            minimum_receive_amount: None,
            label: Some("label".to_string()),
            id: vault_id,
            owner: user_address.clone(),
            destinations: vec![Destination {
                address: user_address.clone(),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            created_at: mock.app.block_info().time,
            status: VaultStatus::Active,
            time_interval: TimeInterval::Hourly,
            balance: Coin::new(
                (vault_deposit - swap_amount).into(),
                DENOM_UOSMO.to_string()
            ),
            slippage_tolerance: None,
            swap_amount,
            pool: Pool {
                pool_id: 0,
                base_denom: DENOM_STAKE.to_string(),
                quote_denom: DENOM_UOSMO.to_string(),
            },
            started_at: Some(mock.app.block_info().time),
            swapped_amount: Coin::new(swap_amount.into(), DENOM_UOSMO.to_string()),
            received_amount: Coin::new(
                (swap_amount - checked_mul(swap_amount, mock.fee_percent).ok().unwrap()).into(),
                DENOM_STAKE.to_string()
            ),
            trigger: Some(TriggerConfiguration::Time {
                target_time: mock
                    .app
                    .block_info()
                    .time
                    .plus_seconds(60 * 60)
                    .minus_nanos(mock.app.block_info().time.subsec_nanos()),
            }),
            dca_plus_config: None,
        }
    );
}

#[test]
fn with_immediate_time_trigger_should_publish_events() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_pass_slippage_tolerance()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DcaVaultCreated {},
            )
            .build(1),
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DcaVaultFundsDeposited {
                    amount: Coin::new(vault_deposit.into(), DENOM_UOSMO),
                },
            )
            .build(2),
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DcaVaultExecutionTriggered {
                    base_denom: DENOM_STAKE.to_string(),
                    quote_denom: DENOM_UOSMO.to_string(),
                    asset_price: Decimal::from_str("1.0").unwrap(),
                },
            )
            .build(3),
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DcaVaultExecutionCompleted {
                    sent: Coin::new(swap_amount.into(), DENOM_UOSMO),
                    received: Coin::new(swap_amount.into(), DENOM_STAKE),
                    fee: Coin::new(
                        (checked_mul(swap_amount, mock.fee_percent).ok().unwrap()).into(),
                        DENOM_STAKE,
                    ),
                },
            )
            .build(4),
        ],
    );
}

#[test]
fn with_immediate_time_trigger_and_slippage_failure_should_update_address_balances() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_fail_slippage_tolerance()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance),
            (&user_address, DENOM_STAKE, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
        ],
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
            (&user_address, DENOM_STAKE, Uint128::zero()),
            (
                &mock.dca_contract_address,
                DENOM_UOSMO,
                ONE_THOUSAND + vault_deposit,
            ),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
        ],
    );
}

#[test]
fn with_immediate_time_trigger_and_slippage_failure_should_update_vault_balance() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_fail_slippage_tolerance()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        user_address,
        Uint128::new(1),
        vault_deposit,
    );
}

#[test]
fn with_time_trigger_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            minimum_receive_amount: None,
            label: Some("label".to_string()),
            id: Uint128::new(1),
            owner: user_address.clone(),
            destinations: vec![Destination {
                address: user_address.clone(),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            created_at: mock.app.block_info().time,
            status: VaultStatus::Scheduled,
            time_interval: TimeInterval::Hourly,
            balance: Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
            slippage_tolerance: None,
            swap_amount,
            pool: Pool {
                pool_id: 0,
                base_denom: DENOM_STAKE.to_string(),
                quote_denom: DENOM_UOSMO.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UOSMO.to_string()),
            received_amount: Coin::new(0, DENOM_STAKE.to_string()),
            trigger: Some(TriggerConfiguration::Time {
                target_time: target_start_time.minus_nanos(target_start_time.subsec_nanos()),
            }),
            dca_plus_config: None,
        }
    );
}

#[test]
fn with_time_trigger_should_update_address_balances() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance),
            (&user_address, DENOM_STAKE, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
        ],
    );

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance - vault_deposit),
            (&user_address, DENOM_STAKE, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UOSMO,
                ONE_THOUSAND + vault_deposit,
            ),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
        ],
    );
}

#[test]
fn with_time_trigger_should_publish_vault_created_event() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DcaVaultCreated {},
        )
        .build(1)],
    );
}

#[test]
fn with_time_trigger_should_publish_funds_deposited_event() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DcaVaultFundsDeposited {
                amount: Coin::new(vault_deposit.into(), DENOM_UOSMO),
            },
        )
        .build(2)],
    );
}

#[test]
fn with_time_trigger_with_existing_vault_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UOSMO)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UOSMO),
            swap_amount,
            "time",
            None,
            None,
        );

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            minimum_receive_amount: None,
            label: Some("label".to_string()),
            id: Uint128::new(2),
            destinations: vec![Destination {
                address: user_address.clone(),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            owner: user_address.clone(),
            created_at: mock.app.block_info().time,
            status: VaultStatus::Scheduled,
            slippage_tolerance: None,
            time_interval: TimeInterval::Hourly,
            balance: Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
            swap_amount,
            pool: Pool {
                pool_id: 0,
                base_denom: DENOM_STAKE.to_string(),
                quote_denom: DENOM_UOSMO.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UOSMO.to_string()),
            received_amount: Coin::new(0, DENOM_STAKE.to_string()),
            trigger: Some(TriggerConfiguration::Time {
                target_time: target_start_time.minus_nanos(target_start_time.subsec_nanos()),
            }),
            dca_plus_config: None,
        }
    );
}

#[test]
fn with_multiple_destinations_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    let mut destinations = vec![];

    for _ in 0..5 {
        destinations.push(Destination {
            address: Addr::unchecked(USER),
            allocation: Decimal::percent(20),
            action: PostExecutionAction::Send,
        });
    }

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: Some(destinations.clone()),
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(
                    (mock.app.block_info().time.seconds() + 10).into(),
                ),
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            minimum_receive_amount: None,
            label: Some("label".to_string()),
            id: Uint128::new(1),
            owner: user_address.clone(),
            destinations,
            created_at: mock.app.block_info().time,
            status: VaultStatus::Scheduled,
            time_interval: TimeInterval::Hourly,
            balance: Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
            slippage_tolerance: None,
            swap_amount,
            pool: Pool {
                pool_id: 0,
                base_denom: DENOM_STAKE.to_string(),
                quote_denom: DENOM_UOSMO.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UOSMO.to_string()),
            received_amount: Coin::new(0, DENOM_STAKE.to_string()),
            trigger: Some(TriggerConfiguration::Time {
                target_time: mock
                    .app
                    .block_info()
                    .time
                    .plus_seconds(10)
                    .minus_nanos(mock.app.block_info().time.subsec_nanos())
            }),
            dca_plus_config: None,
        }
    );
}

#[test]
fn with_passed_in_owner_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    let owner = Addr::unchecked("custom-owner");

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: Some(owner.clone()),
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UOSMO))],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVault {
                vault_id: Uint128::new(1),
            },
        )
        .unwrap();

    assert_eq!(vault_response.vault.owner, owner);
    assert_eq!(vault_response.vault.destinations.len(), 1);
    assert_eq!(
        vault_response.vault.destinations.first().unwrap().address,
        owner
    );
}

#[test]
fn with_insufficient_funds_should_create_inactive_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = Uint128::one();
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            minimum_receive_amount: None,
            label: Some("label".to_string()),
            id: Uint128::new(1),
            owner: user_address.clone(),
            destinations: vec![Destination {
                address: user_address.clone(),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            created_at: mock.app.block_info().time,
            status: VaultStatus::Inactive,
            time_interval: TimeInterval::Hourly,
            balance: Coin::new(vault_deposit.into(), DENOM_UOSMO.to_string()),
            slippage_tolerance: None,
            swap_amount,
            pool: Pool {
                pool_id: 0,
                base_denom: DENOM_STAKE.to_string(),
                quote_denom: DENOM_UOSMO.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UOSMO.to_string()),
            received_amount: Coin::new(0, DENOM_STAKE.to_string()),
            trigger: None,
            dca_plus_config: None,
        }
    );
}

#[test]
fn with_use_dca_plus_true_should_create_dca_plus_config() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance),
            (&user_address, DENOM_STAKE, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
        ],
    );

    let create_vault_response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: Some(true),
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UOSMO))],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&create_vault_response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault = mock
        .app
        .wrap()
        .query_wasm_smart::<VaultResponse>(
            &mock.dca_contract_address,
            &QueryMsg::GetVault { vault_id },
        )
        .unwrap()
        .vault;

    assert_eq!(
        vault.dca_plus_config,
        Some(DcaPlusConfig::new(
            Decimal::percent(5),
            30,
            Coin::new(vault_deposit.into(), vault.get_swap_denom(),),
            vault.get_receive_denom()
        ))
    );
}

#[test]
fn with_large_deposit_should_select_longer_duration_model() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE / Uint128::new(10);
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance),
            (&user_address, DENOM_STAKE, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
        ],
    );

    let create_vault_response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Daily,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: Some(true),
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UOSMO))],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&create_vault_response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(vault_response.vault.dca_plus_config.unwrap().model_id, 80);
}

#[test]
fn with_small_deposit_should_select_shorter_duration_model() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = ONE;
    let swap_amount = TEN;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UOSMO, user_balance),
            (&user_address, DENOM_STAKE, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_STAKE, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UOSMO, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_STAKE, ONE_THOUSAND),
        ],
    );

    let create_vault_response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Daily,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: Some(true),
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UOSMO))],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&create_vault_response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(vault_response.vault.dca_plus_config.unwrap().model_id, 30);
}
