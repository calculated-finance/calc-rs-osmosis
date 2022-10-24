use crate::constants::{ONE, TEN};
use crate::msg::{QueryMsg, VaultsResponse};
use crate::tests::mocks::{fin_contract_unfilled_limit_order, MockApp, DENOM_UKUJI, USER};
use cosmwasm_std::{Addr, Coin, Uint128};

#[test]
fn with_no_vaults_should_succeed() {
    let mock = MockApp::new(fin_contract_unfilled_limit_order());

    let vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: Addr::unchecked("not-a-user".to_string()),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 0);
}

#[test]
fn with_multiple_vaults_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_unfilled_limit_order())
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
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 2);
}

#[test]
fn with_limit_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_unfilled_limit_order())
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
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 1);
    assert_eq!(vaults_response.vaults[0].id, Uint128::new(1));
}

#[test]
fn with_start_after_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_unfilled_limit_order())
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
                start_after: Some(1),
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 1);
    assert_eq!(vaults_response.vaults[0].id, Uint128::new(2));
}

#[test]
fn with_limit_and_start_after_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(3);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_unfilled_limit_order())
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
                start_after: Some(1),
                limit: Some(1),
            },
        )
        .unwrap();

    assert_eq!(vaults_response.vaults.len(), 1);
    assert_eq!(vaults_response.vaults[0].id, Uint128::new(2));
}

#[test]
fn with_invalid_limit_should_fail() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN * Uint128::new(2);
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mock = MockApp::new(fin_contract_unfilled_limit_order())
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
                start_after: None,
                limit: Some(50),
            },
        )
        .unwrap_err();

    assert!(vaults_response
        .to_string()
        .contains("limit cannot be greater than 30."))
}
