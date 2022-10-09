use crate::msg::{ExecuteMsg, QueryMsg, VaultsResponse};
use crate::tests::helpers::{assert_address_balances, assert_response_events};
use crate::tests::mocks::{
    fin_contract_default, fin_contract_partially_filled_order, MockApp, ADMIN, DENOM_UKUJI,
    DENOM_UTEST, USER,
};
use cosmwasm_std::{Addr, Event, Uint128};
use cw_multi_test::Executor;

#[test]
fn when_vault_has_unfulfilled_price_trigger_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default())
        .with_funds_for(&user_address, Uint128::new(100), DENOM_UKUJI)
        .with_vault_with_fin_limit_price_trigger(&user_address, "fin");

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(290)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(210)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    let cancel_vault_by_address_and_id_response = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id: mock.vault_ids["fin"],
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(100)),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(200)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    assert_response_events(
        &cancel_vault_by_address_and_id_response.events,
        &[
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "cancel_vault_by_address_and_id"),
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.fin_contract_address)
                .add_attribute("amount", "10"),
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "after_retract_order")
                .add_attribute("withdraw_required", "false"),
        ],
    );

    let active_vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address.clone(), &QueryMsg::GetVaults {})
        .unwrap();

    assert_eq!(active_vaults_response.vaults.len(), 0);
}

#[test]
fn when_vault_has_partially_filled_price_trigger_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_partially_filled_order())
        .with_funds_for(&user_address, Uint128::new(100), DENOM_UKUJI)
        .with_vault_with_partially_filled_fin_limit_price_trigger(&user_address, "fin");

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(290)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(205)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(205)),
        ],
    );

    let cancel_vault_by_address_and_id_response = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id: mock.vault_ids["fin"],
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(95)),
            (&user_address, DENOM_UTEST, Uint128::new(5)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(200)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    assert_response_events(
        &cancel_vault_by_address_and_id_response.events,
        &[
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "cancel_vault_by_address_and_id"),
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.fin_contract_address)
                .add_attribute("amount", "5"),
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "after_retract_order")
                .add_attribute("withdraw_required", "true"),
        ],
    );

    let active_vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address.clone(), &QueryMsg::GetVaults {})
        .unwrap();

    assert_eq!(active_vaults_response.vaults.len(), 0);
}

#[test]
fn when_vault_has_time_trigger_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default())
        .with_funds_for(&user_address, Uint128::new(100), DENOM_UKUJI)
        .with_vault_with_time_trigger(&user_address, "fin");

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(300)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    let cancel_vault_by_address_and_id_response = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id: mock.vault_ids["fin"],
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(100)),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(200)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    assert_response_events(
        &cancel_vault_by_address_and_id_response.events,
        &[
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "cancel_vault_by_address_and_id")
                .add_attribute("owner", USER)
                .add_attribute("vault_id", "1"),
            Event::new("transfer")
                .add_attribute("recipient", USER)
                .add_attribute("sender", &mock.dca_contract_address)
                .add_attribute("amount", "100ukuji"),
        ],
    );

    let active_vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address.clone(), &QueryMsg::GetVaults {})
        .unwrap();

    assert_eq!(active_vaults_response.vaults.len(), 0);
}
