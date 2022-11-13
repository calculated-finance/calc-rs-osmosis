use crate::{
    contract::{
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID, AFTER_Z_DELEGATION_REPLY_ID,
    },
    handlers::{
        after_fin_limit_order_withdrawn_for_execute_trigger::after_fin_limit_order_withdrawn_for_execute_vault,
        get_events_by_resource_id::get_events_by_resource_id, get_vault::get_vault,
    },
    state::{
        cache::{LimitOrderCache, LIMIT_ORDER_CACHE},
        config::{create_custom_fee, get_config},
        triggers::get_trigger,
    },
    tests::{
        helpers::{
            instantiate_contract, setup_active_vault_with_funds, setup_active_vault_with_low_funds,
        },
        mocks::ADMIN,
    },
};
use base::{
    events::event::{EventBuilder, EventData},
    helpers::math_helpers::checked_mul,
    triggers::trigger::TriggerConfiguration,
    vaults::vault::{PostExecutionAction, VaultStatus},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, BankMsg, Coin, CosmosMsg, Decimal, Reply, SubMsg, SubMsgResponse, SubMsgResult,
    Timestamp, Uint128,
};
use staking_router::msg::ExecuteMsg;
use std::cmp::min;

#[test]
fn after_succcesful_withdrawal_returns_funds_to_destination() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let fee = get_config(&deps.storage).unwrap().swap_fee_percent * vault.get_swap_amount().amount;

    let automation_fee = get_config(&deps.storage).unwrap().delegation_fee_percent;

    let automation_fees = vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .fold(
            Coin::new(0, vault.get_receive_denom()),
            |mut accum, destination| {
                let allocation_amount =
                    checked_mul(vault.get_swap_amount().amount - fee, destination.allocation)
                        .unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, automation_fee).unwrap();
                accum.amount = accum.amount.checked_add(allocation_automation_fee).unwrap();
                accum
            },
        );

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.destinations.first().unwrap().address.to_string(),
        amount: vec![Coin::new(
            (vault.get_swap_amount().amount - fee - automation_fees.amount).into(),
            vault.get_receive_denom()
        )]
    })));
}

#[test]
fn after_succcesful_withdrawal_returns_fee_to_fee_collector() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let fee = config.swap_fee_percent * vault.get_swap_amount().amount;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collector.to_string(),
        amount: vec![Coin::new(fee.into(), vault.get_receive_denom())]
    })));
}

#[test]
fn after_succesful_withdrawal_adjusts_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(
        updated_vault.balance.amount,
        vault.balance.amount - vault.get_swap_amount().amount
    );
}

#[test]
fn after_successful_withdrawal_creates_a_new_time_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
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
fn after_successful_withdrawal_creates_delegation_messages() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let fee = get_config(&deps.storage).unwrap().swap_fee_percent * vault.get_swap_amount().amount;

    let automation_fee = get_config(&deps.storage).unwrap().delegation_fee_percent;

    let automation_fees = vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .fold(
            Coin::new(0, vault.get_receive_denom()),
            |mut accum, destination| {
                let allocation_amount =
                    checked_mul(vault.get_swap_amount().amount - fee, destination.allocation)
                        .unwrap();
                let allocation_automation_fee =
                    checked_mul(allocation_amount, automation_fee).unwrap();
                accum.amount = accum.amount.checked_add(allocation_automation_fee).unwrap();
                accum
            },
        );

    assert!(response.messages.contains(&SubMsg::reply_always(
        CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
            contract_addr: get_config(&deps.storage)
                .unwrap()
                .staking_router_address
                .to_string(),
            msg: to_binary(&ExecuteMsg::ZDelegate {
                delegator_address: vault.owner.clone(),
                validator_address: vault.destinations[0].address.clone(),
                denom: vault.get_receive_denom(),
                amount: checked_mul(
                    vault.get_swap_amount().amount - fee - automation_fees.amount,
                    vault.destinations[0].allocation
                )
                .unwrap()
            })
            .unwrap(),
            funds: vec![]
        }),
        AFTER_Z_DELEGATION_REPLY_ID
    )));
}

#[test]
fn after_successful_withdrawal_creates_execution_completed_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    let config = get_config(&deps.storage).unwrap();

    let fee = Coin::new(
        (config.swap_fee_percent * vault.get_swap_amount().amount).into(),
        vault.get_receive_denom(),
    );

    assert!(events.contains(
        &EventBuilder::new(
            vault.id,
            env.block,
            EventData::DcaVaultExecutionCompleted {
                sent: vault.get_swap_amount(),
                received: Coin::new(
                    vault.get_swap_amount().amount.into(),
                    vault.get_receive_denom()
                ),
                fee: fee,
            },
        )
        .build(1)
    ))
}

#[test]
fn with_empty_resulting_vault_sets_vault_to_inactive() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.status, VaultStatus::Inactive);
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

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let fee_collected = custom_fee_percent * vault.get_swap_amount().amount;

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

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let fee_collected = custom_fee_percent * vault.get_swap_amount().amount;

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

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_withdrawn_for_execute_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let fee_collected =
        min(swap_denom_fee_percent, receive_denom_fee_percent) * vault.get_swap_amount().amount;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collector.to_string(),
        amount: vec![Coin::new(fee_collected.into(), vault.get_receive_denom())]
    })));
}
