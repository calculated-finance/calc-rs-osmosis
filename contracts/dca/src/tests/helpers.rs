use std::str::FromStr;

use super::mocks::{MockApp, ADMIN};
use crate::{
    constants::{ONE, TEN},
    contract::instantiate,
    msg::{EventsResponse, InstantiateMsg, QueryMsg, VaultResponse},
    state::{
        cache::{Cache, CACHE},
        config::FeeCollector,
        pairs::PAIRS,
        triggers::save_trigger,
        vaults::save_vault,
    },
    types::{dca_plus_config::DcaPlusConfig, vault::Vault, vault_builder::VaultBuilder},
};
use base::{
    events::event::Event,
    pair::Pair,
    triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration},
    vaults::vault::{Destination, PostExecutionAction, VaultStatus},
};
use cosmwasm_std::{Addr, Coin, Decimal, DepsMut, Env, MessageInfo, Uint128};

pub fn instantiate_contract(deps: DepsMut, env: Env, info: MessageInfo) {
    let instantiate_message = InstantiateMsg {
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
        fee_collectors: vec![FeeCollector {
            address: "community_pool".to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
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

pub fn setup_vault(
    deps: DepsMut,
    env: Env,
    balance: Coin,
    swap_amount: Uint128,
    is_dca_plus: bool,
) -> Vault {
    let pair = Pair {
        address: Addr::unchecked("pair"),
        base_denom: "base".to_string(),
        quote_denom: "quote".to_string(),
    };

    PAIRS
        .save(deps.storage, pair.address.clone(), &pair)
        .unwrap();

    let owner = Addr::unchecked("owner");

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
            status: VaultStatus::Active,
            pair,
            swap_amount,
            position_type: None,
            slippage_tolerance: None,
            minimum_receive_amount: None,
            balance: balance.clone(),
            time_interval: TimeInterval::Daily,
            started_at: None,
            swapped_amount: Coin {
                denom: "quote".to_string(),
                amount: Uint128::new(0),
            },
            received_amount: Coin {
                denom: "base".to_string(),
                amount: Uint128::new(0),
            },
            dca_plus_config: if is_dca_plus {
                Some(DcaPlusConfig {
                    escrow_level: Decimal::percent(5),
                    model_id: 30,
                    escrowed_balance: Uint128::zero(),
                    total_deposit: balance.amount,
                    standard_dca_swapped_amount: Uint128::zero(),
                    standard_dca_received_amount: Uint128::zero(),
                })
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

    vault
}

pub fn setup_active_vault_with_funds(deps: DepsMut, env: Env) -> Vault {
    setup_vault(deps, env, Coin::new(TEN.into(), "base"), ONE, false)
}

pub fn setup_active_dca_plus_vault_with_funds(deps: DepsMut, env: Env) -> Vault {
    setup_vault(deps, env, Coin::new(TEN.into(), "base"), ONE, true)
}

pub fn setup_active_vault_with_slippage_funds(deps: DepsMut, env: Env) -> Vault {
    setup_vault(
        deps,
        env,
        Coin::new(Uint128::new(500000).into(), "base"),
        Uint128::new(500000),
        false,
    )
}

pub fn setup_active_vault_with_low_funds(deps: DepsMut, env: Env) -> Vault {
    setup_vault(
        deps,
        env,
        Coin::new(Uint128::new(10).into(), "base"),
        Uint128::new(100),
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
        Coin::new(balance.into(), "base"),
        swap_amount,
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
