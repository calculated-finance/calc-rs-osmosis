use crate::constants::{ONE, TEN};
use crate::msg::{ExecuteMsg, QueryMsg, VaultsResponse};
use crate::tests::mocks::{
    fin_contract_filled_limit_order, fin_contract_pass_slippage_tolerance, MockApp, ADMIN,
    DENOM_UKUJI, DENOM_UTEST, USER,
};
use crate::vault::VaultDto;
use base::pair::Pair;
use base::triggers::trigger::{TimeInterval, TriggerConfiguration};
use base::vaults::vault::{Destination, PositionType, PostExecutionAction, VaultStatus};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::Executor;

#[test]
fn with_no_vaults_should_return_all_vaults() {
    let mock = MockApp::new(fin_contract_filled_limit_order());

    let vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: Addr::unchecked("not-a-user".to_string()),
                status: None,
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 0);
}

#[test]
fn with_multiple_vaults_should_return_all_vaults() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
        )
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_2",
        );

    let vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: None,
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 2);
}

#[test]
fn with_one_vault_should_return_proper_vault_data() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_pass_slippage_tolerance())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_time_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
            None,
        );

    let vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: None,
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(
        vaults_response.vaults.first().unwrap(),
        &VaultDto {
            price_threshold: None,
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
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UKUJI.to_string()),
            received_amount: Coin::new(0, DENOM_UTEST.to_string()),
            trigger: Some(TriggerConfiguration::Time {
                target_time: mock
                    .app
                    .block_info()
                    .time
                    .plus_seconds(2)
                    .minus_nanos(mock.app.block_info().time.subsec_nanos())
            })
        }
    );
}

#[test]
fn with_limit_should_return_limited_vaults() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
        )
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_2",
        );

    let vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: None,
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 1);
    assert_eq!(vaults_response.vaults[0].id, Uint128::new(1));
}

#[test]
fn with_start_after_should_return_vaults_after_start_after() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
        )
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_2",
        );

    let vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: None,
                start_after: Some(1),
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 1);
    assert_eq!(vaults_response.vaults[0].id, Uint128::new(2));
}

#[test]
fn with_limit_and_start_after_should_return_limited_vaults_after_start_after() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(3);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
        )
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_2",
        )
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_3",
        );

    let vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: None,
                start_after: Some(1),
                limit: Some(1),
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 1);
    assert_eq!(vaults_response.vaults[0].id, Uint128::new(2));
}

#[test]
fn with_limit_too_large_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
        );

    let vaults_response = mock
        .app
        .wrap()
        .query_wasm_smart::<VaultsResponse>(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: None,
                start_after: None,
                limit: Some(1001),
            },
        )
        .unwrap_err();

    assert!(vaults_response
        .to_string()
        .contains("limit cannot be greater than 1000."))
}

#[test]
fn with_status_filter_should_return_no_vaults() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
        );

    let vaults_response = mock
        .app
        .wrap()
        .query_wasm_smart::<VaultsResponse>(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: Some(VaultStatus::Cancelled),
                start_after: None,
                limit: Some(10),
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 0);
}

#[test]
fn with_status_filter_should_return_all_vaults_with_status() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
        );

    let vaults_response = mock
        .app
        .wrap()
        .query_wasm_smart::<VaultsResponse>(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: Some(VaultStatus::Scheduled),
                start_after: None,
                limit: Some(10),
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 1);
    assert_eq!(vaults_response.vaults[0].status, VaultStatus::Scheduled);
}

#[test]
fn with_status_filter_should_exclude_vaults_without_status() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_filled_limit_order())
        .with_funds_for(&user_address, user_balance, DENOM_UKUJI)
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_1",
        )
        .with_vault_with_filled_fin_limit_price_trigger(
            &user_address,
            None,
            Coin::new(vault_deposit.into(), DENOM_UKUJI),
            swap_amount,
            "fin_2",
        );

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTrigger {
                trigger_id: Uint128::new(1),
            },
            &[],
        )
        .unwrap();

    let vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.clone(),
                status: Some(VaultStatus::Active),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 1);
    assert_eq!(vaults_response.vaults[0].status, VaultStatus::Active);
}
