use crate::msg::{ExecuteMsg, QueryMsg, TriggersResponse};
use crate::tests::helpers::{
    assert_address_balances, assert_response_events, assert_vault_balance,
};
use crate::tests::mocks::{fin_contract_default, MockApp, DENOM_UKUJI, DENOM_UTEST, USER};
use base::triggers::time_configuration::{TimeConfiguration, TimeInterval};
use base::vaults::dca_vault::PositionType;
use cosmwasm_std::{Addr, Coin, Event, Timestamp, Uint128, Uint64};
use cw_multi_test::Executor;

#[test]
fn should_succeed() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(100),
        DENOM_UKUJI,
    );

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

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVaultWithTimeTrigger {
                pair_address: mock.fin_contract_address.to_string(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount: Uint128::new(10),
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
            },
            &vec![Coin {
                denom: String::from(DENOM_UKUJI),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

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

    assert_response_events(
        &response.events,
        &[Event::new("wasm")
            .add_attribute("_contract_addr", &mock.dca_contract_address)
            .add_attribute("method", "create_vault_with_time_trigger")
            .add_attribute("id", "1")
            .add_attribute("owner", USER)
            .add_attribute("vault_id", "1")],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        Uint128::new(100),
    );

    let get_all_time_triggers_response: TriggersResponse<TimeConfiguration> = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetAllTimeTriggers {},
        )
        .unwrap();

    assert_eq!(get_all_time_triggers_response.triggers.len(), 1);

    let trigger = &get_all_time_triggers_response.triggers[0];
    assert_eq!(
        trigger.configuration.target_time,
        Timestamp::from_seconds(target_start_time.seconds())
    );
}
