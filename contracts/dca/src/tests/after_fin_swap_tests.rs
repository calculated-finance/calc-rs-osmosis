use base::{
    events::event::{EventBuilder, EventData, ExecutionSkippedReason},
    pair::Pair,
    triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration},
    vaults::vault::VaultStatus,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Coin, DepsMut, Env, Reply, SubMsgResult, Timestamp, Uint128,
};
use fin_helpers::codes::ERROR_SWAP_SLIPPAGE_EXCEEDED;

use crate::{
    contract::AFTER_FIN_SWAP_REPLY_ID,
    handlers::{
        after_fin_swap::after_fin_swap, get_events_by_resource_id::get_events_by_resource_id,
    },
    state::{
        cache::{Cache, CACHE},
        pairs::PAIRS,
        triggers::{get_trigger, save_trigger},
        vaults::{get_vault, save_vault},
    },
    tests::{helpers::instantiate_contract, mocks::ADMIN},
    types::vault::VaultBuilder,
};

fn setup_vault_with_funds(deps: DepsMut, env: Env) {
    let pair = Pair {
        address: Addr::unchecked("pair"),
        base_denom: "base".to_string(),
        quote_denom: "quote".to_string(),
    };

    PAIRS
        .save(deps.storage, pair.address.clone(), &pair)
        .unwrap();

    let vault = save_vault(
        deps.storage,
        VaultBuilder {
            owner: Addr::unchecked("owner"),
            label: None,
            destinations: vec![],
            created_at: env.block.time.clone(),
            status: VaultStatus::Active,
            pair,
            swap_amount: Uint128::new(100),
            position_type: None,
            slippage_tolerance: None,
            price_threshold: None,
            balance: Coin::new(Uint128::new(1000).into(), "base"),
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
}

fn setup_vault_with_low_funds(deps: DepsMut, env: Env) {
    let pair = Pair {
        address: Addr::unchecked("pair"),
        base_denom: "base".to_string(),
        quote_denom: "quote".to_string(),
    };

    PAIRS
        .save(deps.storage, pair.address.clone(), &pair)
        .unwrap();

    let vault = save_vault(
        deps.storage,
        VaultBuilder {
            owner: Addr::unchecked("owner"),
            label: None,
            destinations: vec![],
            created_at: env.block.time.clone(),
            status: VaultStatus::Active,
            pair,
            swap_amount: Uint128::new(100),
            position_type: None,
            slippage_tolerance: None,
            price_threshold: None,
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
}

#[test]
fn with_insufficient_funds_publishes_unknown_failure_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));
    setup_vault_with_low_funds(deps.as_mut(), env.clone());

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err("Generic failure".to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let vault_id = Uint128::one();

    let events = get_events_by_resource_id(deps.as_ref(), vault_id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(
        &EventBuilder::new(
            vault_id,
            env.block.clone(),
            EventData::DCAVaultExecutionSkipped {
                reason: ExecutionSkippedReason::UnknownFailure
            }
        )
        .build(1)
    ));
}

#[test]
fn with_insufficient_funds_makes_vault_inactive() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));
    setup_vault_with_low_funds(deps.as_mut(), env.clone());
    let vault_id = Uint128::one();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err("Generic failure".to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let vault = get_vault(&mut deps.storage, vault_id).unwrap();

    assert_eq!(vault.status, VaultStatus::Inactive);
}

#[test]
fn with_insufficient_funds_does_not_reduce_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    setup_vault_with_low_funds(deps.as_mut(), env.clone());
    let vault_id = Uint128::one();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err("Generic failure".to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let vault = get_vault(&mut deps.storage, vault_id).unwrap();

    assert_eq!(vault.balance, Coin::new(Uint128::new(10).into(), "base"));
}

#[test]
fn with_insufficient_funds_creates_a_new_time_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    setup_vault_with_low_funds(deps.as_mut(), env.clone());
    let vault_id = Uint128::one();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err("Generic failure".to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let trigger = get_trigger(&mut deps.storage, vault_id).unwrap();

    assert_eq!(
        trigger.unwrap().configuration,
        TriggerConfiguration::Time {
            target_time: Timestamp::from_seconds(env.block.time.seconds() + 60 * 60 * 24)
        }
    );
}

#[test]
fn with_slippage_failure_creates_a_new_time_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    setup_vault_with_funds(deps.as_mut(), env.clone());
    let vault_id = Uint128::one();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err(ERROR_SWAP_SLIPPAGE_EXCEEDED.to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let trigger = get_trigger(&mut deps.storage, vault_id).unwrap();

    assert_eq!(
        trigger.unwrap().configuration,
        TriggerConfiguration::Time {
            target_time: Timestamp::from_seconds(env.block.time.seconds() + 60 * 60 * 24)
        }
    );
}

#[test]
fn with_slippage_failure_publishes_execution_failed_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));
    setup_vault_with_funds(deps.as_mut(), env.clone());
    let vault_id = Uint128::one();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err(ERROR_SWAP_SLIPPAGE_EXCEEDED.to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault_id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(
        &EventBuilder::new(
            vault_id,
            env.block.clone(),
            EventData::DCAVaultExecutionSkipped {
                reason: ExecutionSkippedReason::SlippageToleranceExceeded
            }
        )
        .build(1)
    ));
}

#[test]
fn with_slippage_failure_funds_leaves_vault_active() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    setup_vault_with_funds(deps.as_mut(), env.clone());
    let vault_id = Uint128::one();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err(ERROR_SWAP_SLIPPAGE_EXCEEDED.to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let vault = get_vault(&mut deps.storage, vault_id).unwrap();

    assert_eq!(vault.status, VaultStatus::Active);
}

#[test]
fn with_slippage_failure_does_not_reduce_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    setup_vault_with_funds(deps.as_mut(), env.clone());
    let vault_id = Uint128::one();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err(ERROR_SWAP_SLIPPAGE_EXCEEDED.to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let vault = get_vault(&mut deps.storage, vault_id).unwrap();

    assert_eq!(vault.balance, Coin::new(Uint128::new(1000).into(), "base"));
}
