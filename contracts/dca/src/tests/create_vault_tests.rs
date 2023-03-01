use crate::constants::{ONE, ONE_THOUSAND, TEN, TWO_MICRONS};
use crate::msg::{ExecuteMsg, QueryMsg, VaultResponse};
use crate::state::config::FeeCollector;
use crate::tests::helpers::{
    assert_address_balances, assert_events_published, assert_vault_balance,
};
use crate::tests::mocks::{
    fin_contract_unfilled_limit_order, MockApp, ADMIN, DENOM_UKUJI, DENOM_UTEST, USER,
};
use crate::types::dca_plus_config::DCAPlusConfig;
use crate::types::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::helpers::math_helpers::checked_mul;
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::pair::Pair;
use base::triggers::trigger::{TimeInterval, TriggerConfiguration};
use base::vaults::vault::{Destination, PostExecutionAction, VaultStatus};
use cosmwasm_std::{Addr, Coin, Decimal, Decimal256, Uint128, Uint64};
use cw_multi_test::Executor;
use std::str::FromStr;

use super::mocks::{fin_contract_fail_slippage_tolerance, fin_contract_pass_slippage_tolerance};

#[test]
fn with_price_trigger_should_update_address_balances() {
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
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
                ONE_THOUSAND + user_balance - TWO_MICRONS,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + TWO_MICRONS,
            ),
            (&mock.fin_contract_address, DENOM_UTEST, ONE_THOUSAND),
        ],
    );
}

#[test]
fn with_price_trigger_should_create_vault() {
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
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
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(vault_response.vault.id, Uint128::one());
    assert_eq!(
        vault_response.vault.balance,
        Coin::new((vault_deposit - TWO_MICRONS).into(), DENOM_UKUJI)
    );
    assert_eq!(vault_response.vault.swap_amount, swap_amount);
    assert_eq!(vault_response.vault.pair.address, mock.fin_contract_address);
}

#[test]
fn with_price_trigger_for_fin_buy_should_create_correct_trigger() {
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount * Uint128::new(10)),
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
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
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    match vault_response.vault.trigger.unwrap() {
        TriggerConfiguration::FinLimitOrder {
            target_price,
            order_idx,
        } => {
            assert_eq!(target_price, Decimal256::from_str("0.10").unwrap());
            assert!(order_idx.is_some());
        }
        _ => panic!("expected a fin limit order trigger"),
    }
}

#[test]
fn with_price_trigger_for_fin_sell_should_create_correct_trigger() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UTEST,
    );

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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount * Uint128::new(10)),
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UTEST))],
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

    match vault_response.vault.trigger.unwrap() {
        TriggerConfiguration::FinLimitOrder {
            target_price,
            order_idx,
        } => {
            assert_eq!(target_price, Decimal256::from_str("10.0").unwrap());
            assert!(order_idx.is_some());
        }
        _ => panic!("expected a fin limit order trigger"),
    }
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: Some(swap_amount),
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
fn with_price_trigger_should_publish_funds_deposited_event() {
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: Some(swap_amount),
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
                amount: Coin::new(vault_deposit.into(), DENOM_UKUJI),
            },
        )
        .build(2)],
    );
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
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
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(vault_response.vault.id, Uint128::new(2));
    assert_eq!(
        vault_response.vault.balance,
        Coin::new((vault_deposit - TWO_MICRONS).into(), DENOM_UKUJI)
    );
    assert_eq!(vault_response.vault.swap_amount, swap_amount);
    assert_eq!(vault_response.vault.pair.address, mock.fin_contract_address);
}

#[test]
fn with_price_trigger_twice_for_user_should_succeed() {
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
                ONE_THOUSAND + vault_deposit - TWO_MICRONS,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (&mock.fin_contract_address, DENOM_UKUJI, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + TWO_MICRONS,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
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
                ONE_THOUSAND + vault_deposit + vault_deposit - TWO_MICRONS - TWO_MICRONS,
            ),
            (&mock.dca_contract_address, DENOM_UTEST, ONE_THOUSAND),
            (
                &mock.fin_contract_address,
                DENOM_UKUJI,
                ONE_THOUSAND + TWO_MICRONS, // from newly created fin limit order (unfilled)
            ),
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
                ONE_THOUSAND + TWO_MICRONS, // from initial limit order (filled)
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
        &[EventBuilder::new(
            vault_id,
            mock.app.block_info(),
            EventData::DcaVaultCreated {},
        )
        .build(3)],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        user_address,
        Uint128::new(1),
        vault_deposit - TWO_MICRONS,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    let fee_amount = checked_mul(swap_amount, mock.fee_percent).ok().unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, swap_amount - fee_amount),
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
            (
                &mock.fin_contract_address,
                DENOM_UTEST,
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
        DENOM_UKUJI,
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
        DENOM_UKUJI,
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
                DENOM_UKUJI.to_string()
            ),
            slippage_tolerance: None,
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: Some(mock.app.block_info().time),
            swapped_amount: Coin::new(swap_amount.into(), DENOM_UKUJI.to_string()),
            received_amount: Coin::new(
                (swap_amount - checked_mul(swap_amount, mock.fee_percent).ok().unwrap()).into(),
                DENOM_UTEST.to_string()
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
        DENOM_UKUJI,
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
                    amount: Coin::new(vault_deposit.into(), DENOM_UKUJI),
                },
            )
            .build(2),
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DcaVaultExecutionTriggered {
                    base_denom: DENOM_UTEST.to_string(),
                    quote_denom: DENOM_UKUJI.to_string(),
                    asset_price: Decimal256::from_str("1.0").unwrap(),
                },
            )
            .build(3),
            EventBuilder::new(
                vault_id,
                mock.app.block_info(),
                EventData::DcaVaultExecutionCompleted {
                    sent: Coin::new(swap_amount.into(), DENOM_UKUJI),
                    received: Coin::new(swap_amount.into(), DENOM_UTEST),
                    fee: Coin::new(
                        (checked_mul(swap_amount, mock.fee_percent).ok().unwrap()).into(),
                        DENOM_UTEST,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, user_balance - vault_deposit),
            (&user_address, DENOM_UTEST, Uint128::zero()),
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
fn with_immediate_time_trigger_and_slippage_failure_should_update_vault_balance() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_fail_slippage_tolerance()).with_funds_for(
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
        DENOM_UKUJI,
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string()),
            slippage_tolerance: None,
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UKUJI.to_string()),
            received_amount: Coin::new(0, DENOM_UTEST.to_string()),
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_receive_amount: None,
                use_dca_plus: None,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
        DENOM_UKUJI,
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
                amount: Coin::new(vault_deposit.into(), DENOM_UKUJI),
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
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_receive_amount: None,
                use_dca_plus: None,
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
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string()),
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UKUJI.to_string()),
            received_amount: Coin::new(0, DENOM_UTEST.to_string()),
            trigger: Some(TriggerConfiguration::Time {
                target_time: target_start_time.minus_nanos(target_start_time.subsec_nanos()),
            }),
            dca_plus_config: None,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(
                    mock.app.block_info().time.seconds() - 60,
                )),
                target_receive_amount: None,
                use_dca_plus: None,
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
fn with_multiple_destinations_should_succeed() {
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: Some(destinations.clone()),
                pair_address: mock.fin_contract_address.clone(),
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
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string()),
            slippage_tolerance: None,
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UKUJI.to_string()),
            received_amount: Coin::new(0, DENOM_UTEST.to_string()),
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(
                    mock.app.block_info().time.plus_seconds(2).seconds(),
                )),
                target_receive_amount: Some(swap_amount),
                use_dca_plus: None,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: Addr::unchecked("not-a-pair-address".to_string()),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: Some(vec![Destination {
                    address: Addr::unchecked(USER),
                    allocation: Decimal::percent(50),
                    action: PostExecutionAction::Send,
                }]),
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
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
fn with_destination_allocation_equal_to_zero_should_fail() {
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: Some(vec![
                    Destination {
                        address: Addr::unchecked(USER),
                        allocation: Decimal::percent(0),
                        action: PostExecutionAction::Send,
                    },
                    Destination {
                        address: Addr::unchecked(ADMIN),
                        allocation: Decimal::percent(100),
                        action: PostExecutionAction::Send,
                    },
                ]),
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: all destination allocations must be greater than 0"
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: Some(destinations),
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
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
fn with_passed_in_owner_should_succeed() {
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: None,
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
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
fn with_swap_amount_less_than_50000_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = Uint128::zero();
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
                minimum_receive_amount: None,
                label: Some("label".to_string()),
                destinations: Some(vec![Destination {
                    address: Addr::unchecked(USER),
                    allocation: Decimal::percent(50),
                    action: PostExecutionAction::Send,
                }]),
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: swap amount must be greater than 50000"
    );
}

#[test]
fn when_contract_is_paused_should_fail() {
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
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::UpdateConfig {
                fee_collectors: Some(vec![FeeCollector {
                    address: ADMIN.to_string(),
                    allocation: Decimal::from_str("1").unwrap(),
                }]),
                swap_fee_percent: Some(Decimal::from_str("0.015").unwrap()),
                delegation_fee_percent: Some(Decimal::from_str("0.0075").unwrap()),
                staking_router_address: None,
                page_limit: None,
                paused: Some(true),
                dca_plus_escrow_level: None,
            },
            &[],
        )
        .unwrap();

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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: None,
                target_start_time_utc_seconds: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
        )
        .unwrap_err();

    assert_eq!(
        "Error: contract is paused",
        response.root_cause().to_string()
    )
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
        DENOM_UKUJI,
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UKUJI)],
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
            balance: Coin::new(vault_deposit.into(), DENOM_UKUJI.to_string()),
            slippage_tolerance: None,
            swap_amount,
            pair: Pair {
                address: mock.fin_contract_address.clone(),
                base_denom: DENOM_UTEST.to_string(),
                quote_denom: DENOM_UKUJI.to_string(),
            },
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UKUJI.to_string()),
            received_amount: Coin::new(0, DENOM_UTEST.to_string()),
            trigger: None,
            dca_plus_config: None,
        }
    );
}

#[test]
fn with_adjust_swap_amount_true_should_create_dca_plus_config() {
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: Some(true),
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
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

    assert_eq!(
        vault_response.vault.dca_plus_config,
        Some(DCAPlusConfig {
            escrow_level: Decimal::percent(5),
            model_id: 30,
            escrowed_balance: Uint128::zero(),
            standard_dca_swapped_amount: Uint128::zero(),
            standard_dca_received_amount: Uint128::zero(),
        })
    );
}

#[test]
fn with_long_execution_duration_should_select_longer_duration_model() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE / Uint128::new(10);
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Daily,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: Some(true),
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
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

    assert_eq!(
        vault_response.vault.dca_plus_config,
        Some(DCAPlusConfig {
            escrow_level: Decimal::percent(5),
            model_id: 80,
            escrowed_balance: Uint128::zero(),
            standard_dca_swapped_amount: Uint128::zero(),
            standard_dca_received_amount: Uint128::zero(),
        })
    );
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
                pair_address: mock.fin_contract_address.clone(),
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Daily,
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                use_dca_plus: Some(true),
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
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

    assert_eq!(
        vault_response.vault.dca_plus_config,
        Some(DCAPlusConfig {
            escrow_level: Decimal::percent(5),
            model_id: 30,
            escrowed_balance: Uint128::zero(),
            standard_dca_swapped_amount: Uint128::zero(),
            standard_dca_received_amount: Uint128::zero(),
        })
    );
}
