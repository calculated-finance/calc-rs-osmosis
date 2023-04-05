use super::mocks::{MockApp, ADMIN, DENOM_STAKE, DENOM_UOSMO, FEE_COLLECTOR, USER};
use crate::{
    constants::{ONE, TEN},
    contract::instantiate,
    handlers::get_vault::get_vault,
    msg::{EventsResponse, InstantiateMsg, QueryMsg, VaultResponse},
    state::{
        cache::{Cache, CACHE},
        config::{Config, FeeCollector},
        pools::POOLS,
        triggers::save_trigger,
        vaults::{save_vault, update_vault},
    },
    types::{dca_plus_config::DcaPlusConfig, vault::Vault, vault_builder::VaultBuilder},
};
use base::{
    events::event::Event,
    pool::Pool,
    triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration},
    vaults::vault::{Destination, PostExecutionAction, VaultStatus},
};
use cosmwasm_std::{Addr, Coin, Decimal, DepsMut, Env, MessageInfo, Timestamp, Uint128};
use std::str::FromStr;

pub fn instantiate_contract(deps: DepsMut, env: Env, info: MessageInfo) {
    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        fee_collectors: vec![FeeCollector {
            address: ADMIN.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked("staking-router"),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.0075").unwrap(),
    };

    instantiate(deps, env.clone(), info.clone(), instantiate_message).unwrap();
}

pub fn instantiate_contract_with_community_pool_fee_collector(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) {
    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        fee_collectors: vec![
            FeeCollector {
                address: FEE_COLLECTOR.to_string(),
                allocation: Decimal::from_str("0.5").unwrap(),
            },
            FeeCollector {
                address: "community_pool".to_string(),
                allocation: Decimal::from_str("0.5").unwrap(),
            },
        ],
        swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(ADMIN),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.0075").unwrap(),
    };

    instantiate(deps, env.clone(), info.clone(), instantiate_message).unwrap();
}

pub fn instantiate_contract_with_multiple_fee_collectors(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    fee_collectors: Vec<FeeCollector>,
) {
    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        fee_collectors,
        swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(ADMIN),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.0075").unwrap(),
    };

    instantiate(deps, env.clone(), info.clone(), instantiate_message).unwrap();
}

impl Default for Config {
    fn default() -> Self {
        Self {
            admin: Addr::unchecked(ADMIN),
            fee_collectors: vec![FeeCollector {
                address: ADMIN.to_string(),
                allocation: Decimal::from_str("1").unwrap(),
            }],
            swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
            delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            staking_router_address: Addr::unchecked(ADMIN),
            page_limit: 1000,
            paused: false,
            dca_plus_escrow_level: Decimal::from_str("0.0075").unwrap(),
        }
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self {
            id: Uint128::zero(),
            created_at: Timestamp::default(),
            owner: Addr::unchecked(USER),
            label: Some("vault".to_string()),
            destinations: vec![Destination {
                address: Addr::unchecked(USER),
                allocation: Decimal::percent(100),
                action: PostExecutionAction::Send,
            }],
            status: VaultStatus::Active,
            balance: Coin::new(TEN.into(), DENOM_UOSMO),
            pool: Pool {
                pool_id: 0,
                base_denom: DENOM_UOSMO.to_string(),
                quote_denom: DENOM_STAKE.to_string(),
            },
            swap_amount: ONE,
            slippage_tolerance: None,
            minimum_receive_amount: None,
            time_interval: TimeInterval::Daily,
            started_at: None,
            swapped_amount: Coin::new(0, DENOM_UOSMO),
            received_amount: Coin::new(0, DENOM_STAKE),
            trigger: Some(TriggerConfiguration::Time {
                target_time: Timestamp::from_seconds(0),
            }),
            dca_plus_config: None,
        }
    }
}

impl Default for DcaPlusConfig {
    fn default() -> Self {
        Self {
            escrow_level: Decimal::percent(10),
            model_id: 30,
            total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
            standard_dca_swapped_amount: Coin::new(0, DENOM_UOSMO),
            standard_dca_received_amount: Coin::new(0, DENOM_STAKE),
            escrowed_balance: Coin::new(0, DENOM_STAKE),
        }
    }
}

pub fn setup_new_vault(deps: DepsMut, env: Env, vault: Vault) -> Vault {
    POOLS
        .save(deps.storage, vault.pool.pool_id.clone(), &vault.pool)
        .unwrap();

    update_vault(deps.storage, &vault).unwrap();

    save_trigger(
        deps.storage,
        Trigger {
            vault_id: vault.id,
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time,
            },
        },
    )
    .unwrap();

    get_vault(deps.as_ref(), vault.id).unwrap().vault
}

pub fn setup_vault(
    deps: DepsMut,
    env: Env,
    balance: Uint128,
    swap_amount: Uint128,
    status: VaultStatus,
    slippage_tolerance: Option<Decimal>,
    is_dca_plus: bool,
) -> Vault {
    let pair = Pool {
        pool_id: 0,
        base_denom: DENOM_UOSMO.to_string(),
        quote_denom: DENOM_STAKE.to_string(),
    };

    POOLS
        .save(deps.storage, pair.pool_id.clone(), &pair)
        .unwrap();

    let owner = Addr::unchecked(ADMIN);

    let vault = save_vault(
        deps.storage,
        VaultBuilder {
            owner: owner.clone(),
            label: None,
            destinations: vec![Destination {
                address: owner,
                allocation: Decimal::percent(100),
                action: PostExecutionAction::ZDelegate,
            }],
            created_at: env.block.time.clone(),
            status,
            pool: pair,
            swap_amount,
            position_type: None,
            slippage_tolerance,
            minimum_receive_amount: None,
            balance: Coin::new(balance.into(), DENOM_UOSMO),
            time_interval: TimeInterval::Daily,
            started_at: None,
            swapped_amount: Coin {
                denom: DENOM_UOSMO.to_string(),
                amount: Uint128::new(0),
            },
            received_amount: Coin {
                denom: DENOM_STAKE.to_string(),
                amount: Uint128::new(0),
            },
            dca_plus_config: if is_dca_plus {
                Some(DcaPlusConfig::new(
                    Decimal::percent(5),
                    30,
                    Coin::new(balance.into(), DENOM_UOSMO),
                    DENOM_STAKE.to_string(),
                ))
            } else {
                None
            },
        },
    )
    .unwrap();

    save_trigger(
        deps.storage,
        Trigger {
            vault_id: vault.id,
            configuration: TriggerConfiguration::Time {
                target_time: env.block.time,
            },
        },
    )
    .unwrap();

    CACHE
        .save(
            deps.storage,
            &Cache {
                vault_id: vault.id,
                owner: Addr::unchecked("owner"),
            },
        )
        .unwrap();

    get_vault(deps.as_ref(), vault.id).unwrap().vault
}

pub fn setup_active_vault_with_funds(deps: DepsMut, env: Env) -> Vault {
    setup_vault(deps, env, TEN, ONE, VaultStatus::Active, None, false)
}

pub fn setup_active_dca_plus_vault_with_funds(deps: DepsMut, env: Env) -> Vault {
    setup_vault(deps, env, TEN, ONE, VaultStatus::Active, None, true)
}

pub fn setup_active_vault_with_slippage_funds(deps: DepsMut, env: Env) -> Vault {
    setup_vault(
        deps,
        env,
        Uint128::new(500000),
        Uint128::new(500000),
        VaultStatus::Active,
        None,
        false,
    )
}

pub fn setup_active_vault_with_low_funds(deps: DepsMut, env: Env) -> Vault {
    setup_vault(
        deps,
        env,
        Uint128::new(10),
        Uint128::new(100),
        VaultStatus::Active,
        None,
        false,
    )
}

pub fn setup_active_dca_plus_vault_with_low_funds(
    deps: DepsMut,
    env: Env,
    balance: Uint128,
    swap_amount: Uint128,
) -> Vault {
    setup_vault(
        deps,
        env,
        balance,
        swap_amount,
        VaultStatus::Active,
        None,
        true,
    )
}

pub fn assert_address_balances(mock: &MockApp, address_balances: &[(&Addr, &str, Uint128)]) {
    address_balances
        .iter()
        .for_each(|(address, denom, expected_balance)| {
            assert_eq!(
                mock.get_balance(address, denom),
                expected_balance,
                "Balance mismatch for {} at {}",
                address,
                denom
            );
        })
}

pub fn assert_events_published(mock: &MockApp, resource_id: Uint128, expected_events: &[Event]) {
    let events_response: EventsResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            &mock.dca_contract_address,
            &QueryMsg::GetEventsByResourceId {
                resource_id,
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    expected_events.iter().for_each(|expected_event| {
        assert!(
            events_response.events.contains(expected_event),
            "Expected actual_events: \n\n{:?}\n\nto contain event:\n\n{:?}\n\n but it wasn't found",
            events_response.events,
            expected_event
        );
    });
}

pub fn assert_vault_balance(
    mock: &MockApp,
    contract_address: &Addr,
    address: Addr,
    vault_id: Uint128,
    balance: Uint128,
) {
    let vault_response: VaultResponse = mock
        .app
        .wrap()
        .query_wasm_smart(contract_address, &QueryMsg::GetVault { vault_id })
        .unwrap();

    let vault = &vault_response.vault;

    assert_eq!(
        vault.balance.amount, balance,
        "Vault balance mismatch for vault_id: {}, owner: {}",
        vault_id, address
    );
}
