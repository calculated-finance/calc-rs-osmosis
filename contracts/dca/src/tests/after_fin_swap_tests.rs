use base::{
    events::event::{EventBuilder, EventData, ExecutionSkippedReason},
    triggers::trigger::TriggerConfiguration,
    vaults::vault::VaultStatus,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, Coin, Event, Reply, SubMsg, SubMsgResponse, SubMsgResult, Timestamp, Uint128,
};
use fin_helpers::codes::ERROR_SWAP_SLIPPAGE_EXCEEDED;

use crate::{
    contract::AFTER_FIN_SWAP_REPLY_ID,
    handlers::{
        after_fin_swap::after_fin_swap, get_events_by_resource_id::get_events_by_resource_id,
    },
    state::{config::get_config, triggers::get_trigger, vaults::get_vault},
    tests::{
        helpers::{
            instantiate_contract, setup_active_vault_with_funds, setup_active_vault_with_low_funds,
        },
        mocks::ADMIN,
    },
};

#[test]
fn with_succcesful_swap_returns_funds_to_destination() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    let response = after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm-trade")
                    .add_attribute("base_amount", vault.get_swap_amount().amount.to_string())
                    .add_attribute("quote_amount", receive_amount.to_string())],
                data: None,
            }),
        },
    )
    .unwrap();

    let fee = get_config(&deps.storage).unwrap().fee_percent * receive_amount;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.destinations.first().unwrap().address.to_string(),
        amount: vec![Coin::new(
            (receive_amount - fee).into(),
            vault.get_receive_denom()
        )]
    })));
}

#[test]
fn with_succcesful_swap_returns_fee_to_fee_collector() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    let response = after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm-trade")
                    .add_attribute("base_amount", vault.get_swap_amount().amount.to_string())
                    .add_attribute("quote_amount", receive_amount.to_string())],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let fee = config.fee_percent * receive_amount;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collector.to_string(),
        amount: vec![Coin::new(fee.into(), vault.get_receive_denom())]
    })));
}

#[test]
fn with_succcesful_swap_adjusts_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm-trade")
                    .add_attribute("base_amount", vault.get_swap_amount().amount.to_string())
                    .add_attribute("quote_amount", receive_amount.to_string())],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

    assert_eq!(
        updated_vault.balance.amount,
        vault.balance.amount - vault.get_swap_amount().amount
    );
}

#[test]
fn with_insufficient_funds_publishes_unknown_failure_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

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
    setup_active_vault_with_low_funds(deps.as_mut(), env.clone());
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

    setup_active_vault_with_low_funds(deps.as_mut(), env.clone());
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

    setup_active_vault_with_low_funds(deps.as_mut(), env.clone());
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

    setup_active_vault_with_funds(deps.as_mut(), env.clone());
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
    setup_active_vault_with_funds(deps.as_mut(), env.clone());
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

    setup_active_vault_with_funds(deps.as_mut(), env.clone());
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

    setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let vault_id = Uint128::one();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err(ERROR_SWAP_SLIPPAGE_EXCEEDED.to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let vault = get_vault(&mut deps.storage, vault_id).unwrap();

    assert_eq!(vault.balance, Coin::new(Uint128::new(1000).into(), "base"));
}
