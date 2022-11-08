use std::str::FromStr;

use base::{
    helpers::message_helpers::get_flat_map_for_event_type, triggers::trigger::TimeInterval,
};
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::Executor;

use crate::{
    constants::{ONE, TEN},
    msg::{ExecuteMsg, QueryMsg, VaultResponse},
    tests::mocks::{fin_contract_unfilled_limit_order, MockApp, DENOM_UKUJI, USER},
};

#[test]
fn should_succeed() {
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
                time_interval: TimeInterval::Daily,
                target_receive_amount: None,
                target_start_time_utc_seconds: None,
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::UpdateVault {
                address: Addr::unchecked(USER),
                vault_id,
                label: Some("test".to_string()),
            },
            &vec![],
        )
        .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(&mock.dca_contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    assert_eq!(vault_response.vault.label, Some("test".to_string()))
}

#[test]
fn cancelled_vault_should_fail() {
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
                time_interval: TimeInterval::Daily,
                target_receive_amount: None,
                target_start_time_utc_seconds: None,
            },
            &vec![Coin::new(vault_deposit.into(), String::from(DENOM_UKUJI))],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CancelVault {
                vault_id: vault_id.clone(),
            },
            &vec![],
        )
        .unwrap();

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::UpdateVault {
                address: Addr::unchecked(USER),
                vault_id,
                label: None,
            },
            &vec![],
        )
        .unwrap_err();

    assert_eq!(
        "Error: vault is already cancelled",
        response.root_cause().to_string()
    );
}
