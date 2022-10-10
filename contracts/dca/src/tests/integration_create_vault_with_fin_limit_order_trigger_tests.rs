use crate::msg::{EventsResponse, ExecuteMsg, QueryMsg};
use crate::tests::helpers::{
    assert_address_balances, assert_response_events, assert_vault_balance,
};
use crate::tests::mocks::{fin_contract_default, MockApp, DENOM_UKUJI, DENOM_UTEST, USER};
use base::events::event::{EventBuilder, EventData};
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::triggers::time_configuration::TimeInterval;
use base::vaults::dca_vault::PositionType;
use cosmwasm_std::{Addr, Coin, Decimal256, Uint128};
use cw_multi_test::Executor;
use std::str::FromStr;

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

    mock.app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVaultWithFINLimitOrderTrigger {
                pair_address: mock.fin_contract_address.to_string(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount: Uint128::new(10),
                time_interval: TimeInterval::Hourly,
                target_price: Decimal256::from_str("1.0").unwrap(),
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
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(290)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(210)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    let vault_id = Uint128::new(1);

    let events_response: EventsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetEventsByAddressAndResourceId {
                address: user_address.to_string(),
                resource_id: vault_id,
            },
        )
        .unwrap();

    assert_response_events(
        &events_response.events,
        &[EventBuilder::new(user_address.clone(), vault_id, EventData::VaultCreated).build(1)],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        vault_id,
        Uint128::new(100),
    );
}

#[test]
fn twice_for_user_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default())
        .with_funds_for(&user_address, Uint128::new(200), DENOM_UKUJI)
        .with_vault_with_fin_limit_price_trigger(&user_address, "fin");

    assert_address_balances(
        &mock,
        &[
            (&user_address, DENOM_UKUJI, Uint128::new(100)),
            (&user_address, DENOM_UTEST, Uint128::new(0)),
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(290)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(210)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    let create_vault_response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVaultWithFINLimitOrderTrigger {
                pair_address: mock.fin_contract_address.to_string(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount: Uint128::new(10),
                time_interval: TimeInterval::Hourly,
                target_price: Decimal256::from_str("1.0").unwrap(),
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
            (&mock.dca_contract_address, DENOM_UKUJI, Uint128::new(380)),
            (&mock.dca_contract_address, DENOM_UTEST, Uint128::new(200)),
            (&mock.fin_contract_address, DENOM_UKUJI, Uint128::new(220)),
            (&mock.fin_contract_address, DENOM_UTEST, Uint128::new(200)),
        ],
    );

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&create_vault_response.events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let events_response: EventsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetEventsByAddressAndResourceId {
                address: user_address.to_string(),
                resource_id: vault_id,
            },
        )
        .unwrap();

    assert_response_events(
        &events_response.events,
        &[EventBuilder::new(user_address.clone(), vault_id, EventData::VaultCreated).build(2)],
    );

    assert_vault_balance(
        &mock,
        &mock.dca_contract_address,
        &user_address,
        Uint128::new(1),
        Uint128::new(100),
    );
}
