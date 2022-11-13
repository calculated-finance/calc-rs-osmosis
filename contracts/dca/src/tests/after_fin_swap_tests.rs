use std::cmp::min;

use base::{
    events::event::{EventBuilder, EventData, ExecutionSkippedReason},
    helpers::math_helpers::checked_mul,
    triggers::trigger::TriggerConfiguration,
    vaults::vault::{PostExecutionAction, VaultStatus},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, Coin, Decimal, Event, Reply, SubMsg, SubMsgResponse, SubMsgResult, Timestamp, Uint128,
};
use fin_helpers::codes::ERROR_SWAP_SLIPPAGE_EXCEEDED;

use crate::{
    constants::TEN,
    contract::AFTER_FIN_SWAP_REPLY_ID,
    handlers::{
        after_fin_swap::after_fin_swap, get_events_by_resource_id::get_events_by_resource_id,
    },
    state::{
        config::{create_custom_fee, get_config},
        triggers::get_trigger,
        vaults::get_vault,
    },
    tests::{
        helpers::{
            instantiate_contract, setup_active_vault_with_funds, setup_active_vault_with_low_funds,
            setup_active_vault_with_slippage_funds,
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

    let fee = get_config(&deps.storage).unwrap().swap_fee_percent * receive_amount;

    let automation_fee = get_config(&deps.storage).unwrap().delegation_fee_percent;

    let automation_fees = vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .fold(
            Coin::new(0, vault.get_receive_denom()),
            |mut accum, destination| {
                let allocation_amount =
                    checked_mul(receive_amount - fee, destination.allocation).unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, automation_fee).unwrap();
                accum.amount = accum.amount.checked_add(allocation_automation_fee).unwrap();
                accum
            },
        );

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.destinations.first().unwrap().address.to_string(),
        amount: vec![Coin::new(
            (receive_amount - fee - automation_fees.amount).into(),
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
    let fee = config.swap_fee_percent * receive_amount;

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
fn with_successful_swap_creates_a_new_time_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    after_fin_swap(
        deps.as_mut(),
        env.clone(),
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

    let trigger = get_trigger(&mut deps.storage, vault.id).unwrap();

    assert_eq!(
        trigger.unwrap().configuration,
        TriggerConfiguration::Time {
            target_time: Timestamp::from_seconds(env.block.time.seconds() + 60 * 60 * 24)
        }
    );
}

#[test]
fn with_successful_swap_and_insufficient_remaining_funds_sets_vault_to_inactive() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    after_fin_swap(
        deps.as_mut(),
        env.clone(),
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

    let vault = get_vault(&deps.storage, vault.id).unwrap();

    assert_eq!(vault.status, VaultStatus::Inactive);
}

#[test]
fn with_failed_swap_and_insufficient_funds_does_not_reduce_vault_balance() {
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
fn with_failed_swap_and_insufficient_funds_does_not_create_a_new_time_trigger() {
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
    assert!(trigger.is_none());
}

#[test]
fn with_failed_swap_and_insufficient_funds_sets_vault_to_inactive() {
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

    assert_eq!(vault.status, VaultStatus::Inactive);
}

#[test]
fn with_failed_swap_and_insufficient_funds_publishes_skipped_event_with_unknown_failure() {
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

    let events = get_events_by_resource_id(deps.as_ref(), vault_id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(
        &EventBuilder::new(
            vault_id,
            env.block.clone(),
            EventData::DcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::UnknownFailure
            }
        )
        .build(1)
    ));
}

#[test]
fn with_failed_swap_creates_a_new_time_trigger() {
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
fn with_failed_swap_publishes_skipped_event_with_slippage_failure() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));
    setup_active_vault_with_slippage_funds(deps.as_mut(), env.clone());
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
            EventData::DcaVaultExecutionSkipped {
                reason: ExecutionSkippedReason::SlippageToleranceExceeded
            }
        )
        .build(1)
    ));
}

#[test]
fn with_failed_swap_leaves_vault_active() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    setup_active_vault_with_slippage_funds(deps.as_mut(), env.clone());
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
fn with_failed_swap_does_not_reduce_vault_balance() {
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

    assert_eq!(vault.balance, Coin::new(TEN.into(), "base"));
}

#[test]
fn with_custom_fee_for_base_denom_takes_custom_fee() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let custom_fee_percent = Decimal::percent(20);

    create_custom_fee(
        &mut deps.storage,
        vault.get_swap_denom(),
        custom_fee_percent,
    )
    .unwrap();

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
    let fee_collected = custom_fee_percent * receive_amount;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collector.to_string(),
        amount: vec![Coin::new(fee_collected.into(), vault.get_receive_denom())]
    })));
}

#[test]
fn with_custom_fee_for_quote_denom_takes_custom_fee() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let custom_fee_percent = Decimal::percent(20);

    create_custom_fee(
        &mut deps.storage,
        vault.get_receive_denom(),
        custom_fee_percent,
    )
    .unwrap();

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
    let fee_collected = custom_fee_percent * receive_amount;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collector.to_string(),
        amount: vec![Coin::new(fee_collected.into(), vault.get_receive_denom())]
    })));
}

#[test]
fn with_custom_fee_for_both_denoms_takes_lower_fee() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let swap_denom_fee_percent = Decimal::percent(20);
    let receive_denom_fee_percent = Decimal::percent(40);

    create_custom_fee(
        &mut deps.storage,
        vault.get_swap_denom(),
        swap_denom_fee_percent,
    )
    .unwrap();

    create_custom_fee(
        &mut deps.storage,
        vault.get_receive_denom(),
        receive_denom_fee_percent,
    )
    .unwrap();

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
    let fee_collected = min(swap_denom_fee_percent, receive_denom_fee_percent) * receive_amount;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collector.to_string(),
        amount: vec![Coin::new(fee_collected.into(), vault.get_receive_denom())]
    })));
}
