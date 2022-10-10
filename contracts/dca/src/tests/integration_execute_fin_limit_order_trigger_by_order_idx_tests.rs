use crate::msg::{ExecuteMsg, QueryMsg, TriggersResponse, VaultResponse};
use crate::tests::helpers::{
    assert_address_balances, assert_response_events, assert_vault_balance,
};
use crate::tests::mocks::{
    fin_contract_default, fin_contract_partially_filled_order, MockApp, ADMIN, DENOM_UKUJI,
    DENOM_UTEST, USER,
};
use base::triggers::time_configuration::{TimeConfiguration, TimeInterval};
use cosmwasm_std::{Addr, Coin, Event, Uint128};
use cw_multi_test::Executor;

#[test]
fn should_succeed() {
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

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &&QueryMsg::GetVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id: mock.vault_ids.get("fin").unwrap().vault_id,
            },
        )
        .unwrap();

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteFINLimitOrderTriggerByOrderIdx {
                order_idx: vault_response.vault.trigger_id,
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(0)),
            (&user_address, DENOM_UTEST, Uint128::new(10)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(290)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(210)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(190)),
        ],
    );

    assert_response_events(
        &response.events,
        &[
            Event::new("wasm")
                .add_attribute("_contract_addr", mock.dca_contract_address.to_string())
                .add_attribute("method", "execute_fin_limit_order_trigger_by_order_idx"),
            Event::new("wasm")
                .add_attribute("_contract_addr", mock.dca_contract_address.to_string())
                .add_attribute("method", "after_withdraw_order")
                .add_attribute("trigger_id", "2"),
        ],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        Uint128::new(90),
    );

    let get_all_time_triggers_response: TriggersResponse<TimeConfiguration> = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetTimeTriggers {},
        )
        .unwrap();

    assert_eq!(get_all_time_triggers_response.triggers.len(), 1);

    // TODO: assert vault executions are accurate
}

#[test]
fn when_order_partially_filled_should_fail() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_partially_filled_order())
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

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &&QueryMsg::GetVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id: mock.vault_ids.get("fin").unwrap().vault_id,
            },
        )
        .unwrap();

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteFINLimitOrderTriggerByOrderIdx {
                order_idx: vault_response.vault.trigger_id,
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: fin limit order has not been completely filled"
    );

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

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        Uint128::new(100),
    );
}

#[test]
fn when_executions_result_in_empty_vault_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default())
        .with_funds_for(&user_address, Uint128::new(100), DENOM_UKUJI)
        .with_price_trigger_vault(
            &user_address,
            Coin {
                denom: DENOM_UKUJI.to_string(),
                amount: Uint128::new(15),
            },
            Uint128::new(10),
            TimeInterval::Daily,
            "fin",
        );

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(85)),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(205)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(210)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    let vault_with_price_trigger_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &&QueryMsg::GetVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id: mock.vault_ids.get("fin").unwrap().vault_id,
            },
        )
        .unwrap();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteFINLimitOrderTriggerByOrderIdx {
                order_idx: vault_with_price_trigger_response.vault.trigger_id,
            },
            &[],
        )
        .unwrap();

    let vault_with_time_trigger_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &&QueryMsg::GetVaultByAddressAndId {
                address: user_address.to_string(),
                vault_id: mock.vault_ids.get("fin").unwrap().vault_id,
            },
        )
        .unwrap();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::ExecuteTimeTriggerById {
                trigger_id: vault_with_time_trigger_response.vault.trigger_id,
            },
            &[],
        )
        .unwrap();

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(85)),
            (&user_address, DENOM_UTEST, Uint128::new(15)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(200)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(215)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(185)),
        ],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        vault_with_time_trigger_response.vault.id,
        Uint128::new(0),
    );
}
