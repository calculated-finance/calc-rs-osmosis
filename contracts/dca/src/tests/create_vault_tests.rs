use crate::msg::{ExecuteMsg, QueryMsg, TriggersResponse, VaultResponse};
use crate::tests::helpers::{
    assert_address_balances, assert_events_published, assert_vault_balance,
};
use crate::tests::mocks::{fin_contract_default, MockApp, DENOM_UKUJI, DENOM_UTEST, USER};
use base::events::event::{EventBuilder, EventData};
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::pair::Pair;
use base::triggers::trigger::TimeInterval;
use base::vaults::vault::{PositionType, Vault, VaultConfiguration, VaultStatus};
use cosmwasm_std::{Addr, Coin, Decimal256, Uint128, Uint64};
use cw_multi_test::Executor;
use std::str::FromStr;

#[test]
fn with_price_trigger_should_update_address_balances() {
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
fn with_price_trigger_should_create_vault() {
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

    let response = mock
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

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
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
            id: Uint128::new(1),
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
            trigger_id: Some(Uint128::new(1)),
        }
    );
}

#[test]
fn with_price_trigger_with_existing_vault_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default())
        .with_funds_for(&user_address, Uint128::new(200), DENOM_UKUJI)
        .with_vault_with_fin_limit_price_trigger(&user_address, "fin");

    let response = mock
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

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
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
fn with_price_trigger_should_publish_vault_created_event() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(100),
        DENOM_UKUJI,
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
                target_start_time_utc_seconds: None,
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
            },
            &vec![Coin {
                denom: String::from(DENOM_UKUJI),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::VaultCreated).build(1)],
    );
}

#[test]
fn with_price_trigger_twice_for_user_should_succeed() {
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
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::VaultCreated).build(2)],
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
fn with_time_trigger_should_update_address_balances() {
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
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_price: None,
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
}

#[test]
fn with_time_trigger_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(100),
        DENOM_UKUJI,
    );

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

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
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_price: None,
            },
            &vec![Coin::new(100, DENOM_UKUJI)],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

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
            id: Uint128::new(1),
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
            trigger_id: Some(Uint128::new(1)),
        }
    );
}

#[test]
fn with_time_trigger_with_existing_vault_should_create_vault() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default())
        .with_funds_for(&user_address, Uint128::new(200), DENOM_UKUJI)
        .with_vault_with_time_trigger(&user_address, "time");

    let target_start_time = mock.app.block_info().time.plus_seconds(2);

    let response = mock
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
                target_start_time_utc_seconds: Some(Uint64::from(target_start_time.seconds())),
                target_price: None,
            },
            &vec![Coin::new(100, DENOM_UKUJI)],
        )
        .unwrap();

    let vault_id = Uint128::from_str(
        &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
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
fn with_time_trigger_should_publish_vault_created_event() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(100),
        DENOM_UKUJI,
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
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin {
                denom: String::from(DENOM_UKUJI),
                amount: Uint128::new(100),
            }],
        )
        .unwrap();

    let vault_id = Uint128::new(1);

    assert_events_published(
        &mock,
        vault_id,
        &[EventBuilder::new(vault_id, mock.app.block_info(), EventData::VaultCreated).build(1)],
    );
}

#[test]
fn with_time_trigger_with_no_target_time_should_succeed() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(100),
        DENOM_UKUJI,
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
                target_start_time_utc_seconds: None,
                target_price: None,
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

    let get_all_time_triggers_response: TriggersResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address.clone(),
            &QueryMsg::GetTimeTriggers {},
        )
        .unwrap();

    assert_eq!(get_all_time_triggers_response.triggers.len(), 1);

    let trigger = &get_all_time_triggers_response.triggers[0]
        .configuration
        .to_owned()
        .into_time()
        .unwrap();

    assert_eq!(trigger.1.seconds(), mock.app.block_info().time.seconds());
}

#[test]
fn with_time_trigger_with_target_time_in_the_past_should_fail() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(100),
        DENOM_UKUJI,
    );

    let response = mock
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
                target_start_time_utc_seconds: Some(Uint64::from(
                    mock.app.block_info().time.seconds() - 60,
                )),
                target_price: None,
            },
            &vec![Coin {
                denom: String::from(DENOM_UKUJI),
                amount: Uint128::new(100),
            }],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: target_start_time_utc_seconds must be some time in the future"
    );
}

#[test]
fn with_price_and_time_trigger_should_fail() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(50),
        DENOM_UKUJI,
    );

    let response = mock
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
                target_start_time_utc_seconds: Some(Uint64::from(
                    mock.app.block_info().time.plus_seconds(2).seconds(),
                )),
                target_price: Some(Decimal256::from_str("1.0").unwrap()),
            },
            &vec![],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: Cannot provide both a target_start_time_utc_seconds and a target_price"
    );
}

#[test]
fn with_no_assets_should_fail() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(50),
        DENOM_UKUJI,
    );

    let response = mock
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
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: received 0 denoms but required exactly 1"
    );
}

#[test]
fn with_multiple_assets_should_fail() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default())
        .with_funds_for(&user_address, Uint128::new(50), DENOM_UKUJI)
        .with_funds_for(&user_address, Uint128::new(50), DENOM_UTEST);

    let response = mock
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
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin::new(10, DENOM_UTEST), Coin::new(10, DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "Error: received 2 denoms but required exactly 1"
    );
}

#[test]
fn with_non_existent_pair_address_should_fail() {
    let user_address = Addr::unchecked(USER);
    let mut mock = MockApp::new(fin_contract_default()).with_funds_for(
        &user_address,
        Uint128::new(100),
        DENOM_UKUJI,
    );

    let response = mock
        .app
        .execute_contract(
            Addr::unchecked(USER),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::CreateVault {
                pair_address: "not-a-pair-address".to_string(),
                position_type: PositionType::Enter,
                slippage_tolerance: None,
                swap_amount: Uint128::new(10),
                time_interval: TimeInterval::Hourly,
                target_start_time_utc_seconds: None,
                target_price: None,
            },
            &vec![Coin::new(10, DENOM_UKUJI)],
        )
        .unwrap_err();

    assert_eq!(
        response.root_cause().to_string(),
        "base::pair::Pair not found"
    );
}
