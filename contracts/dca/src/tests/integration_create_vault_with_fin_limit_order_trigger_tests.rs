use crate::msg::{ExecuteMsg, QueryMsg, VaultResponse};
use crate::tests::helpers::{
    assert_address_balances, assert_response_events, assert_vault_balance,
};
use crate::tests::mocks::{fin_contract_default, MockApp, DENOM_UKUJI, DENOM_UTEST, USER};
use base::helpers::message_helpers::find_value_for_key_in_wasm_event_with_method;
use base::triggers::time_configuration::TimeInterval;
use base::vaults::dca_vault::PositionType;
use cosmwasm_std::{Addr, Coin, Decimal256, Event, Uint128};
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
            &QueryMsg::GetVault {
                address: user_address.to_string(),
                vault_id: Uint128::new(1),
            },
        )
        .unwrap();

    assert_response_events(
        &create_vault_response.events,
        &[
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "create_vault_with_fin_limit_order_trigger")
                .add_attribute("owner", USER)
                .add_attribute("vault_id", vault_response.vault.id.to_string()),
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "after_submit_order")
                .add_attribute("trigger_id", vault_response.vault.trigger_id.to_string()),
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
fn create_second_for_user_should_succeed() {
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

    let vault_id = find_value_for_key_in_wasm_event_with_method(
        &create_vault_response.events,
        "create_vault_with_fin_limit_order_trigger",
        "vault_id",
    );

    let trigger_id = find_value_for_key_in_wasm_event_with_method(
        &create_vault_response.events,
        "after_submit_order",
        "trigger_id",
    );

    assert_response_events(
        &create_vault_response.events,
        &[
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "create_vault_with_fin_limit_order_trigger")
                .add_attribute("owner", USER)
                .add_attribute("vault_id", vault_id),
            Event::new("wasm")
                .add_attribute("_contract_addr", &mock.dca_contract_address)
                .add_attribute("method", "after_submit_order")
                .add_attribute("trigger_id", trigger_id),
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
