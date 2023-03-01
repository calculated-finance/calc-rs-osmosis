use super::mocks::{fin_contract_unfilled_limit_order, MockApp, DENOM_UKUJI, USER};
use crate::{
    constants::{ONE, TEN},
    msg::{ExecuteMsg, QueryMsg, TriggerIdResponse, VaultResponse},
};
use base::{
    helpers::message_helpers::get_flat_map_for_event_type, triggers::trigger::TimeInterval,
};
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_multi_test::Executor;
use std::str::FromStr;

#[test]
fn should_fetch_existing_trigger_id_by_order_idx() {
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
                target_receive_amount: Some(swap_amount),
                target_start_time_utc_seconds: None,
                dca_plus_direction: None,
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

    let trigger_id_response: TriggerIdResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetTriggerIdByFinLimitOrderIdx {
                order_idx: vault_response
                    .vault
                    .trigger
                    .unwrap()
                    .into_fin_limit_order()
                    .unwrap()
                    .1
                    .unwrap()
                    .clone(),
            },
        )
        .unwrap();

    assert_eq!(trigger_id_response.trigger_id, vault_id);
}
