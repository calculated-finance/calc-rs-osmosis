use super::mocks::{fin_contract_filled_limit_order, MockApp, DENOM_UOSMO, USER};
use crate::{
    constants::{ONE, TEN},
    msg::{ExecuteMsg, QueryMsg, TriggerIdsResponse},
};
use base::triggers::trigger::TimeInterval;
use cosmwasm_std::{Addr, Coin, Uint64};
use cw_multi_test::Executor;

#[test]
fn should_return_active_triggers_only() {
    let user_address = Addr::unchecked(USER);
    let user_balance = TEN;
    let vault_deposit = TEN;
    let swap_amount = ONE;
    let mut mock = MockApp::new(fin_contract_filled_limit_order()).with_funds_for(
        &user_address,
        user_balance,
        DENOM_UOSMO,
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
                pool_id: 0,
                position_type: None,
                slippage_tolerance: None,
                swap_amount,
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(
                    mock.app.block_info().time.seconds() + 100,
                )),
                target_receive_amount: None,
                use_dca_plus: None,
            },
            &vec![Coin::new(vault_deposit.into(), DENOM_UOSMO)],
        )
        .unwrap();

    let before_get_time_trigger_ids_response: TriggerIdsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.dca_contract_address.clone(),
            &QueryMsg::GetTimeTriggerIds { limit: None },
        )
        .unwrap();

    assert_eq!(before_get_time_trigger_ids_response.trigger_ids.len(), 0);

    mock.elapse_time(200);

    let after_get_time_trigger_ids_response: TriggerIdsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.dca_contract_address.clone(),
            &QueryMsg::GetTimeTriggerIds { limit: None },
        )
        .unwrap();

    assert_eq!(after_get_time_trigger_ids_response.trigger_ids.len(), 1);
}
