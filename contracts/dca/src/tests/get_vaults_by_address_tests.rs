use crate::constants::{ONE, TEN};
use crate::msg::{QueryMsg, VaultsResponse};
use crate::tests::mocks::{fin_contract_unfilled_limit_order, MockApp, DENOM_UKUJI, USER};
use cosmwasm_std::{Addr, Coin, Uint128};

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

    let vault_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: user_address.to_string(),
            },
        )
        .unwrap();

    assert_eq!(vault_response.vaults.len(), 2);
}

#[test]
fn with_no_vaults_should_succeed() {
    let mock = MockApp::new(fin_contract_unfilled_limit_order());

    let vault_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultsByAddress {
                address: "not-a-user".to_string(),
            },
        )
        .unwrap();

    assert_eq!(vault_response.vaults.len(), 0);
}
