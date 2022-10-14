use crate::msg::{ExecuteMsg, QueryMsg, VaultsResponse};
use crate::tests::helpers::{assert_address_balances, assert_events_published};
use crate::tests::mocks::{
    fin_contract_default, fin_contract_partially_filled_order, MockApp, ADMIN, DENOM_UKUJI,
    DENOM_UTEST, USER,
};
use base::events::event::{EventBuilder, EventData};
use cosmwasm_std::{Addr, Uint128};
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

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id: mock.vault_ids.get("fin").unwrap().to_owned(),
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

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::VaultCancelled).build(2)],
    );

    let active_vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVaultsByAddress {
                address: user_address.to_string(),
            },
        )
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

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id,
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

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::VaultCancelled).build(2)],
    );

    let active_vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVaultsByAddress {
                address: user_address.to_string(),
            },
        )
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

    let vault_id = mock.vault_ids.get("fin").unwrap().to_owned();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id,
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

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::VaultCancelled).build(2)],
    );

    let active_vaults_response: VaultsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetVaultsByAddress {
                address: user_address.to_string(),
            },
        )
        .unwrap();

    assert_eq!(active_vaults_response.vaults.len(), 0);
}
