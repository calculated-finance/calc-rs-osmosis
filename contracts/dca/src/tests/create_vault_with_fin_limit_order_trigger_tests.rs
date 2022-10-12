use crate::msg::{ExecuteMsg, QueryMsg, VaultResponse};
use crate::tests::helpers::{
    assert_address_balances, assert_events_published, assert_vault_balance,
};
use crate::tests::mocks::{fin_contract_default, MockApp, DENOM_UKUJI, DENOM_UTEST, USER};
use base::events::event::{EventBuilder, EventData};
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::pair::Pair;
use base::triggers::trigger::TimeInterval;
use base::vaults::vault::{PositionType, Vault, VaultConfiguration, VaultStatus};
use cosmwasm_std::{Addr, Coin, Decimal256, Uint128};
use cw_multi_test::Executor;
use std::str::FromStr;

#[test]
fn should_update_address_balances() {
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
            &ExecuteMsg::CreateVault {
                pair_address: mock.fin_contract_address.to_string(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount: Uint128::new(10),
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
                target_start_time_utc_seconds: None,
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
}

#[test]
fn should_create_vault() {
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
            &ExecuteMsg::CreateVault {
                pair_address: mock.fin_contract_address.to_string(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount: Uint128::new(10),
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
                target_start_time_utc_seconds: None,
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

    assert_events_published(
        &mock,
        vault_id,
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
fn with_existing_vault_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default())
        .with_funds_for(&user_address, Uint128::new(200), DENOM_UKUJI)
        .with_vault_with_fin_limit_price_trigger(&user_address, "fin");

    let response = mock.app.execute_contract(
        Addr::unchecked(USER),
        mock.dca_contract_address.clone(),
        &ExecuteMsg::CreateVault {
            pair_address: mock.fin_contract_address.to_string(),
            position_type: PositionType::Enter,
            slippage_tolerance: None,
            swap_amount: Uint128::new(10),
            time_interval: TimeInterval::Hourly,
            target_price: Some(Decimal256::from_str("1.0").unwrap()),
            target_start_time_utc_seconds: None,
        },
        &vec![Coin {
            denom: String::from(DENOM_UKUJI),
            amount: Uint128::new(100),
        }],
    );

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.unwrap().events, "wasm").unwrap()["vault_id"],
    )
    .unwrap();

    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetVaultById { vault_id },
        )
        .unwrap();

    assert_eq!(
        vault_response.vault,
        Vault {
            id: Uint128::new(2),
            owner: user_address.clone(),
            created_at: mock.app.block_info().time,
            balances: vec![Coin::new(100, DENOM_UKUJI.to_string())],
            status: VaultStatus::Active,
            configuration: VaultConfiguration::DCA {
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount: Uint128::new(10),
                pair: Pair {
                    address: mock.fin_contract_address.clone(),
                    base_denom: DENOM_UTEST.to_string(),
                    quote_denom: DENOM_UKUJI.to_string(),
                },
            },
            trigger_id: Some(Uint128::new(2)),
        }
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
            &ExecuteMsg::CreateVault {
                pair_address: mock.fin_contract_address.to_string(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount: Uint128::new(10),
                time_interval: TimeInterval::Hourly,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
                target_start_time_utc_seconds: None,
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

    assert_events_published(
        &mock,
        vault_id,
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
