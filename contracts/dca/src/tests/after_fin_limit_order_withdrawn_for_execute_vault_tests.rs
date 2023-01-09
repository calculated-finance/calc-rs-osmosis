use crate::{
    constants::TWO_MICRONS,
    contract::{
        AFTER_BANK_SWAP_REPLY_ID, AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
        AFTER_FIN_SWAP_REPLY_ID, AFTER_Z_DELEGATION_REPLY_ID,
    },
    handlers::{
        after_fin_limit_order_withdrawn_for_execute_trigger::after_fin_limit_order_withdrawn_for_execute_vault,
        get_events_by_resource_id::get_events_by_resource_id, get_vault::get_vault,
    },
    state::{
        cache::{LimitOrderCache, LIMIT_ORDER_CACHE},
        config::{create_custom_fee, get_config, FeeCollector},
        fin_limit_order_change_timestamp::FIN_LIMIT_ORDER_CHANGE_TIMESTAMP,
        triggers::get_trigger,
    },
    tests::{
        helpers::{
            instantiate_contract, setup_active_vault_with_funds, setup_active_vault_with_low_funds,
            setup_vault, instantiate_contract_with_multiple_fee_collectors,
        },
        mocks::{ADMIN, DENOM_UKUJI},
    },
};
use base::{
    events::event::{EventBuilder, EventData},
    triggers::trigger::TriggerConfiguration,
    vaults::vault::{PostExecutionAction, VaultStatus}, helpers::math_helpers::checked_mul,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, BankMsg, Coin, CosmosMsg, Decimal, Decimal256, Reply, SubMsg, SubMsgResponse,
    SubMsgResult, Timestamp, Uint128, WasmMsg, Addr,
};
use kujira::fin::ExecuteMsg as FINExecuteMsg;
use staking_router::msg::ExecuteMsg;
use std::{cmp::min, str::FromStr};

#[test]
fn after_succcesful_withdrawal_of_new_limit_order_invokes_a_fin_swap() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));
    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - TWO_MICRONS).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(TWO_MICRONS.into(), vault.get_receive_denom()),
        ],
    );

    FIN_LIMIT_ORDER_CHANGE_TIMESTAMP
        .save(deps.as_mut().storage, &env.block.time.minus_seconds(10))
        .unwrap();

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: TWO_MICRONS,
                filled: TWO_MICRONS,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - TWO_MICRONS).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    assert!(response.messages.contains(&SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: vault.pair.address.to_string(),
            msg: to_binary(&FINExecuteMsg::Swap {
                offer_asset: None,
                belief_price: None,
                max_spread: None,
                to: None,
            })
            .unwrap(),
            funds: vec![vault.get_swap_amount()]
        }),
        AFTER_FIN_SWAP_REPLY_ID
    )));
}

#[test]
fn after_succcesful_withdrawal_returns_funds_to_destination() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));
    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let config = get_config(deps.as_ref().storage).unwrap();

    let automation_fee_rate = config.delegation_fee_percent
        * vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum::<Decimal>();

    let swap_fee = received_amount * config.swap_fee_percent;
    let total_after_swap_fee = received_amount - swap_fee;
    let automation_fee = total_after_swap_fee * automation_fee_rate;
    let total_fee = swap_fee + automation_fee;
    let total_after_total_fee = received_amount - total_fee;

    let destination = vault.destinations.first().unwrap();
    let disbursement = Coin::new(
        (destination.allocation * total_after_total_fee).into(),
        vault.get_receive_denom(),
    );

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        BankMsg::Send {
            to_address: destination.address.to_string(),
            amount: vec![disbursement]
        },
        AFTER_BANK_SWAP_REPLY_ID
    )));
}

#[test]
fn after_succcesful_withdrawal_of_new_limit_order_returns_limit_order_to_fee_collector() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - TWO_MICRONS).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(TWO_MICRONS.into(), vault.get_receive_denom()),
        ],
    );

    FIN_LIMIT_ORDER_CHANGE_TIMESTAMP
        .save(deps.as_mut().storage, &env.block.time.minus_seconds(10))
        .unwrap();

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: TWO_MICRONS,
                filled: TWO_MICRONS,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - TWO_MICRONS).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let config = get_config(deps.as_ref().storage).unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(TWO_MICRONS.into(), vault.get_receive_denom())]
    })));
}

#[test]
fn after_succcesful_withdrawal_returns_fees_to_fee_collector() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let automation_fee_rate = config.delegation_fee_percent
        * vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum::<Decimal>();

    let swap_fee = received_amount * config.swap_fee_percent;
    let total_after_swap_fee = received_amount - swap_fee;
    let automation_fee = total_after_swap_fee * automation_fee_rate;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(swap_fee.into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(automation_fee.into(), vault.get_receive_denom())]
    })));
}

#[test]
fn after_succcesful_withdrawal_returns_fees_to_multiple_fee_collectors() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let fee_allocation = Decimal::from_str("0.5").unwrap();

    instantiate_contract_with_multiple_fee_collectors(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]), vec![FeeCollector {
        address: Addr::unchecked(ADMIN),
        allocation: fee_allocation,
    },
    FeeCollector {
        address: Addr::unchecked("fee-collector-two"),
        allocation: fee_allocation,
    }]);
    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let automation_fee_rate = config.delegation_fee_percent
        * vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum::<Decimal>();

    let swap_fee = received_amount * config.swap_fee_percent;
    let total_after_swap_fee = received_amount - swap_fee;
    let automation_fee = total_after_swap_fee * automation_fee_rate;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(
            checked_mul(swap_fee, fee_allocation).unwrap().into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(checked_mul(automation_fee, fee_allocation).unwrap().into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[1].address.to_string(),
        amount: vec![Coin::new(
            checked_mul(swap_fee, fee_allocation).unwrap().into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[1].address.to_string(),
        amount: vec![Coin::new(checked_mul(automation_fee, fee_allocation).unwrap().into(), vault.get_receive_denom())]
    })));
}

#[test]
fn after_succesful_withdrawal_adjusts_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                vault.get_swap_amount().amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                vault.get_swap_amount().amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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
fn after_successful_withdrawal_resulting_in_low_funds_does_not_create_a_new_time_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Coin::new(100000, DENOM_UKUJI),
        Uint128::new(60000),
    );

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                vault.get_swap_amount().amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(vault.trigger, None);
}

#[test]
fn after_successful_withdrawal_creates_delegation_messages() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let config = get_config(&deps.storage).unwrap();

    let automation_fee_rate = config.delegation_fee_percent
        * vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum::<Decimal>();

    let swap_fee = received_amount * config.swap_fee_percent;
    let total_after_swap_fee = received_amount - swap_fee;
    let automation_fee = total_after_swap_fee * automation_fee_rate;
    let total_fee = swap_fee + automation_fee;
    let total_after_total_fee = received_amount - total_fee;

    let destination = vault.destinations.first().unwrap();

    assert!(response.messages.contains(&SubMsg::reply_always(
        CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
            contract_addr: get_config(&deps.storage)
                .unwrap()
                .staking_router_address
                .to_string(),
            msg: to_binary(&ExecuteMsg::ZDelegate {
                delegator_address: vault.owner.clone(),
                validator_address: destination.address.clone(),
                denom: vault.get_receive_denom(),
                amount: total_after_total_fee * destination.allocation
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

    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let config = get_config(&deps.storage).unwrap();

    let automation_fee_rate = config.delegation_fee_percent
        * vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum::<Decimal>();

    let swap_fee = received_amount * config.swap_fee_percent;
    let total_after_swap_fee = received_amount - swap_fee;
    let automation_fee = total_after_swap_fee * automation_fee_rate;
    let total_fee = swap_fee + automation_fee;

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(
        &EventBuilder::new(
            vault.id,
            env.block,
            EventData::DcaVaultExecutionCompleted {
                sent: vault.get_swap_amount(),
                received: Coin::new(received_amount.into(), vault.get_receive_denom()),
                fee: Coin::new(total_fee.into(), vault.get_receive_denom()),
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

    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let automation_fee_rate = config.delegation_fee_percent
        * vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum::<Decimal>();

    let swap_fee = received_amount * custom_fee_percent;
    let total_after_swap_fee = received_amount - swap_fee;
    let automation_fee = total_after_swap_fee * automation_fee_rate;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(swap_fee.into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(automation_fee.into(), vault.get_receive_denom())]
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

    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let automation_fee_rate = config.delegation_fee_percent
        * vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum::<Decimal>();

    let swap_fee = received_amount * custom_fee_percent;
    let total_after_swap_fee = received_amount - swap_fee;
    let automation_fee = total_after_swap_fee * automation_fee_rate;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(swap_fee.into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(automation_fee.into(), vault.get_receive_denom())]
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

    let received_amount = vault.get_swap_amount().amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - vault.get_swap_amount().amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(
                    (vault.balance.amount - vault.get_swap_amount().amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
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

    let automation_fee_rate = config.delegation_fee_percent
        * vault
            .destinations
            .iter()
            .filter(|destination| destination.action == PostExecutionAction::ZDelegate)
            .map(|destination| destination.allocation)
            .sum::<Decimal>();

    let swap_fee = received_amount * min(swap_denom_fee_percent, receive_denom_fee_percent);
    let total_after_swap_fee = received_amount - swap_fee;
    let automation_fee = total_after_swap_fee * automation_fee_rate;

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(swap_fee.into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(automation_fee.into(), vault.get_receive_denom())]
    })));
}
