use crate::constants::{ONE, ONE_THOUSAND, TEN};
use crate::msg::{ExecuteMsg, QueryMsg, TriggerIdsResponse, VaultResponse};
use crate::tests::helpers::{
    assert_address_balances, assert_events_published, assert_vault_balance,
};
use crate::tests::mocks::{
    fin_contract_unfilled_limit_order, MockApp, DENOM_UKUJI, DENOM_UTEST, USER,
};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::pair::Pair;
use base::triggers::trigger::{TimeInterval, TriggerConfiguration};
use base::vaults::vault::{Destination, PositionType, PostExecutionAction, VaultStatus};
use cosmwasm_std::{Addr, Coin, Decimal, Decimal256, Uint128, Uint64};
use cw_multi_test::Executor;
use std::str::FromStr;

#[test]
fn with_fin_limit_order_trigger_should_update_address_balances() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
                target_start_time_utc_seconds: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string())],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + user_balance - swap_amount,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + swap_amount,
            ),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );
}

#[test]
fn with_fin_limit_order_trigger_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
                target_start_time_utc_seconds: None,
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVault {
                vault_id,
                address: user_address.clone(),
            },
        )
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            price_threshold: None,
            label: "label".to_string(),
            id: Uint128::new(1),
            owner: user_address.clone(),
            destinations: vec![Destination {
                address: user_address.clone(),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            created_at: mock.app.block_info().time,
            status: VaultStatus::Active,
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string()),
            position_type: PositionType::Enter,
            time_interval: TimeInterval::Hourly,
            slippage_tolerance: None,
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None
        }
    );
}

#[test]
fn with_fin_limit_order_trigger_should_create_trigger() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.879").unwrap()),
                target_start_time_utc_seconds: None,
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVault {
                vault_id,
                address: user_address.clone(),
            },
        )
        .unwrap();

    match vault_response.trigger.configuration {
        TriggerConfiguration::FINLimitOrder {
            target_price,
            order_idx,
        } => {
            assert_eq!(target_price, Decimal256::from_str("1.879").unwrap());
            assert!(order_idx.is_some());
        }
        _ => panic!("expected a fin limit order trigger"),
    }
}

#[test]
fn with_price_trigger_with_existing_vault_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
                target_start_time_utc_seconds: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVault {
                vault_id,
                address: user_address.clone(),
            },
        )
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            price_threshold: None,
            label: "label".to_string(),
            id: Uint128::new(2),
            owner: user_address.clone(),
            destinations: vec![Destination {
                address: user_address.clone(),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            created_at: mock.app.block_info().time,
            status: VaultStatus::Active,
            position_type: PositionType::Enter,
            time_interval: TimeInterval::Hourly,
            slippage_tolerance: None,
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None
        }
    );
}

#[test]
fn with_price_trigger_should_publish_vault_created_event() {
    let user_address = Addr::unchecked(USER);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        vault_deposit,
        DENOM_UKUJI,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::DCAVaultCreated).build(1)],
    );
}

#[test]
fn with_fin_limit_order_trigger_twice_for_user_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin",
        );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit - swap_amount,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + swap_amount,
            ),
        ],
    );

    let create_vault_response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
                target_start_time_utc_seconds: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string())],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit + vault_deposit - swap_amount - swap_amount,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + swap_amount, // from newly created fin limit order (unfilled)
            ),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + swap_amount, // from initial limit order (filled)
            ),
        ],
    );

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&create_vault_response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::DCAVaultCreated).build(2)],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        user_address,
        Uint128::new(1),
        vault_deposit,
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
        DENOM_UKUJI,
    );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
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
        DENOM_UKUJI,
    );

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVault {
                vault_id,
                address: user_address.clone(),
            },
        )
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            price_threshold: None,
            label: "label".to_string(),
            id: Uint128::new(1),
            owner: user_address.clone(),
            destinations: vec![Destination {
                address: user_address.clone(),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            created_at: mock.app.block_info().time,
            status: VaultStatus::Active,
            position_type: PositionType::Enter,
            time_interval: TimeInterval::Hourly,
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string()),
            slippage_tolerance: None,
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None
        }
    );
}

#[test]
fn with_time_trigger_with_existing_vault_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "time",
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
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVault {
                vault_id,
                address: user_address.clone(),
            },
        )
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            price_threshold: None,
            label: "label".to_string(),
            id: Uint128::new(2),
            destinations: vec![Destination {
                address: user_address.clone(),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send
            }],
            owner: user_address.clone(),
            created_at: mock.app.block_info().time,
            status: VaultStatus::Active,
            position_type: PositionType::Enter,
            slippage_tolerance: None,
            time_interval: TimeInterval::Hourly,
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string()),
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None
        }
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
        DENOM_UKUJI,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::DCAVaultCreated).build(1)],
    );
}

#[test]
fn with_time_trigger_with_no_target_time_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (
                &mock.dca_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + vault_deposit,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );

    let get_all_time_triggers_response: TriggerIdsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetTimeTriggerIds {},
        )
        .unwrap();

    assert_eq!(get_all_time_triggers_response.trigger_ids.len(), 1);
}

#[test]
fn with_mulitple_destinations_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
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
                price_threshold: None,
                label: "label".to_string(),
                destinations: Some(destinations.clone()),
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVault {
                vault_id,
                address: user_address.clone(),
            },
        )
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            price_threshold: None,
            label: "label".to_string(),
            id: Uint128::new(1),
            owner: user_address.clone(),
            destinations,
            created_at: mock.app.block_info().time,
            status: VaultStatus::Active,
            position_type: PositionType::Enter,
            time_interval: TimeInterval::Hourly,
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string()),
            slippage_tolerance: None,
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None
        }
    );
}

#[test]
fn with_time_trigger_with_target_time_in_the_past_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(
                    mock.app.block_info().time.seconds() - 60,
                )),
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: target_start_time_utc_seconds must be some time in the future"
    );
}

#[test]
fn with_price_and_time_trigger_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(
                    mock.app.block_info().time.plus_seconds(2).seconds(),
                )),
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: cannot provide both a target_start_time_utc_seconds and a target_price"
    );
}

#[test]
fn with_no_assets_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: received 0 denoms but required exactly 1"
    );
}

#[test]
fn with_multiple_assets_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_funds_for(&user_address, user_balance, DENOM_UTEST);

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![
                Coin::new(vault_deposit.into(), DENOM_UTEST),
                Coin::new(vault_deposit.into(), DENOM_UKUJI),
            ],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: received 2 denoms but required exactly 1"
    );
}

#[test]
fn with_non_existent_pair_address_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: Addr::unchecked("not-a-pair-address".to_string()),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "base::pair::Pair not found"
    );
}

#[test]
fn with_destination_allocations_less_than_100_percent_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: Some(vec![Destination {
                    address: Addr::unchecked(USER),
                    allocation: Decimal::percent(50),
                    action: PostExecutionAction::Send,
                }]),
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: destination allocations must add up to 1"
    );
}

#[test]
fn with_more_than_10_destination_allocations_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    let mut destinations = vec![];

    for _ in 0..20 {
        destinations.push(Destination {
            address: Addr::unchecked(USER),
            allocation: Decimal::percent(5),
            action: PostExecutionAction::Send,
        });
    }

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: None,
                price_threshold: None,
                label: "label".to_string(),
                destinations: Some(destinations),
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: no more than 10 destinations can be provided"
    );
}

#[test]
fn with_passed_in_owner_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UKUJI,
    );

    let owner = Addr::unchecked("custom-owner");

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                owner: Some(owner.clone()),
                price_threshold: None,
                label: "label".to_string(),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
                target_start_time_utc_seconds: None,
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVault {
                vault_id: Uint128::new(1),
                address: owner.clone(),
            },
        )
        .unwrap();

    assert_eq!(vault_response.vault.owner, owner);
}
