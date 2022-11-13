use std::str::FromStr;

use super::mocks::{MockApp, ADMIN};
use crate::{
    constants::{ONE, TEN},
    contract::instantiate,
    msg::{EventsResponse, InstantiateMsg, QueryMsg, VaultResponse},
    state::{
        cache::{Cache, CACHE},
        pairs::PAIRS,
        triggers::save_trigger,
        vaults::save_vault,
    },
    types::{vault::Vault, vault_builder::VaultBuilder},
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
        fee_collector: Addr::unchecked(ADMIN),
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(ADMIN),
        page_limit: 1000,
        paused: false,
    };

    let _instantiate_result =
        instantiate(deps, env.clone(), info.clone(), instantiate_message).unwrap();
}

pub fn setup_active_vault_with_funds(deps: DepsMut, env: Env) -> Vault {
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
            swap_amount: ONE,
            position_type: None,
            slippage_tolerance: None,
            minimum_receive_amount: None,
            balance: Coin::new(TEN.into(), "base"),
            time_interval: TimeInterval::Daily,
            started_at: None,
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

pub fn setup_active_vault_with_slippage_funds(deps: DepsMut, env: Env) -> Vault {
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
                action: PostExecutionAction::Send,
            }],
            created_at: env.block.time.clone(),
            status: VaultStatus::Active,
            pair,
            swap_amount: Uint128::new(500000),
            position_type: None,
            slippage_tolerance: None,
            minimum_receive_amount: None,
            balance: Coin::new(Uint128::new(500000).into(), "base"),
            time_interval: TimeInterval::Daily,
            started_at: None,
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

pub fn setup_active_vault_with_low_funds(deps: DepsMut, env: Env) -> Vault {
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
                action: PostExecutionAction::Send,
            }],
            created_at: env.block.time.clone(),
            status: VaultStatus::Active,
            pair,
            swap_amount: Uint128::new(100),
            position_type: None,
            slippage_tolerance: None,
            minimum_receive_amount: None,
            balance: Coin::new(Uint128::new(10).into(), "base"),
            time_interval: TimeInterval::Daily,
            started_at: None,
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
