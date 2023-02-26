use base::{
    events::event::{EventBuilder, EventData, ExecutionSkippedReason},
    helpers::{community_pool::create_fund_community_pool_msg, math_helpers::checked_mul},
    triggers::trigger::{Trigger, TriggerConfiguration},
    vaults::vault::{PostExecutionAction, VaultStatus},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, Coin, Decimal, Decimal256, Reply, SubMsg, SubMsgResponse, SubMsgResult, Timestamp,
    Uint128,
};
use fin_helpers::codes::ERROR_SWAP_SLIPPAGE_EXCEEDED;
use std::{cmp::min, str::FromStr};

use crate::{
    constants::TEN,
    contract::{AFTER_BANK_SWAP_REPLY_ID, AFTER_FIN_SWAP_REPLY_ID},
    handlers::{
        after_fin_swap::after_fin_swap, get_events_by_resource_id::get_events_by_resource_id,
    },
    helpers::vault_helpers::get_swap_amount,
    state::{
        cache::{SwapCache, SWAP_CACHE},
        config::{create_custom_fee, get_config, FeeCollector},
        triggers::{delete_trigger, get_trigger, save_trigger},
        vaults::get_vault,
    },
    tests::{
        helpers::{
            instantiate_contract, instantiate_contract_with_multiple_fee_collectors,
            setup_active_vault_with_funds, setup_active_vault_with_low_funds,
            setup_active_vault_with_slippage_funds, setup_vault,
        },
        mocks::{ADMIN, DENOM_UKUJI},
    },
};

#[test]
fn with_succcesful_swap_returns_funds_to_destination() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(10000);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    let response = after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
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

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        BankMsg::Send {
            to_address: vault.destinations.first().unwrap().address.to_string(),
            amount: vec![Coin::new(
                (receive_amount - fee - automation_fees.amount).into(),
                vault.get_receive_denom(),
            )],
        },
        AFTER_BANK_SWAP_REPLY_ID,
    )));
}

#[test]
fn with_succcesful_swap_returns_fee_to_fee_collector() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    let response = after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let swap_fee = config.swap_fee_percent * receive_amount;
    let total_after_swap_fee = receive_amount - swap_fee;

    let automation_fee = vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .fold(Uint128::zero(), |acc, destination| {
            let allocation_amount =
                checked_mul(total_after_swap_fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            acc.checked_add(allocation_automation_fee).unwrap()
        });

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
fn with_succcesful_swap_returns_fee_to_multiple_fee_collectors() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let fee_allocation = Decimal::from_str("0.5").unwrap();

    instantiate_contract_with_multiple_fee_collectors(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vec![
            FeeCollector {
                address: ADMIN.to_string(),
                allocation: fee_allocation,
            },
            FeeCollector {
                address: "community_pool".to_string(),
                allocation: fee_allocation,
            },
        ],
    );

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    let response = after_fin_swap(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let swap_fee = config.swap_fee_percent * receive_amount;
    let total_after_swap_fee = receive_amount - swap_fee;

    let automation_fee = vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .fold(Uint128::zero(), |acc, destination| {
            let allocation_amount =
                checked_mul(total_after_swap_fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            acc.checked_add(allocation_automation_fee).unwrap()
        });

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(
            checked_mul(swap_fee, fee_allocation).unwrap().into(),
            vault.get_receive_denom()
        )]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(
            checked_mul(automation_fee, fee_allocation).unwrap().into(),
            vault.get_receive_denom()
        )]
    })));

    assert!(response
        .messages
        .contains(&SubMsg::new(create_fund_community_pool_msg(
            env.contract.address.to_string(),
            vec![Coin::new(
                checked_mul(swap_fee, fee_allocation).unwrap().into(),
                vault.get_receive_denom()
            )]
        ))));

    assert!(response
        .messages
        .contains(&SubMsg::new(create_fund_community_pool_msg(
            env.contract.address.to_string(),
            vec![Coin::new(
                checked_mul(automation_fee, fee_allocation).unwrap().into(),
                vault.get_receive_denom()
            )]
        ))));
}

#[test]
fn with_succcesful_swap_adjusts_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

    assert_eq!(
        updated_vault.balance.amount,
        vault.balance.amount
            - get_swap_amount(vault.clone(), &deps.as_ref())
                .unwrap()
                .amount
    );
}

#[test]
fn with_succcesful_swap_adjusts_swapped_amount_stat() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount
                    - get_swap_amount(vault.clone(), &deps.as_ref())
                        .unwrap()
                        .amount)
                    .into(),
                vault.get_swap_denom(),
            ),
            Coin::new(receive_amount.into(), vault.get_receive_denom()),
        ],
    );

    after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

    assert_eq!(
        updated_vault.swapped_amount.amount,
        get_swap_amount(vault.clone(), &deps.as_ref())
            .unwrap()
            .amount
    );
}

#[test]
fn with_succcesful_swap_adjusts_received_amount_stat() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();
    let config = get_config(&deps.storage).unwrap();

    let mut fee = config.swap_fee_percent * receive_amount;

    vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .for_each(|destination| {
            let allocation_amount =
                checked_mul(receive_amount - fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            fee = fee.checked_add(allocation_automation_fee).unwrap();
        });

    assert_eq!(updated_vault.received_amount.amount, receive_amount - fee);
}

#[test]
fn with_successful_swap_creates_a_new_time_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    after_fin_swap(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
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
fn with_successful_swap_resulting_in_low_funds_sets_vault_to_inactive() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Coin::new(100000, DENOM_UKUJI),
        Uint128::new(60000),
    );

    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    after_fin_swap(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let vault = get_vault(&deps.storage, vault.id).unwrap();

    assert_eq!(vault.status, VaultStatus::Inactive);
}

#[test]
fn with_successful_swap_resulting_in_low_funds_does_not_create_time_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Coin::new(100000, DENOM_UKUJI),
        Uint128::new(60000),
    );

    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    after_fin_swap(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let vault = get_vault(&deps.storage, vault.id).unwrap();

    assert_eq!(vault.trigger, None);
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
fn with_failed_swap_and_low_funds_and_price_trigger_creates_new_time_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_slippage_funds(deps.as_mut(), env.clone());

    delete_trigger(deps.as_mut().storage, vault.id).unwrap();

    save_trigger(
        deps.as_mut().storage,
        Trigger {
            vault_id: vault.id,
            configuration: TriggerConfiguration::FinLimitOrder {
                target_price: Decimal256::one(),
                order_idx: Some(Uint128::one()),
            },
        },
    )
    .unwrap();

    let reply = Reply {
        id: AFTER_FIN_SWAP_REPLY_ID,
        result: SubMsgResult::Err(ERROR_SWAP_SLIPPAGE_EXCEEDED.to_string()),
    };

    after_fin_swap(deps.as_mut(), env.clone(), reply).unwrap();

    let vault = get_vault(&mut deps.storage, vault.id).unwrap();

    assert_eq!(
        vault.trigger.clone(),
        Some(TriggerConfiguration::Time {
            target_time: Timestamp::from_seconds(env.block.time.seconds() + 60 * 60 * 24)
        })
    );
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

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    let response = after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let swap_fee = custom_fee_percent * receive_amount;
    let total_after_swap_fee = receive_amount - swap_fee;

    let automation_fee = vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .fold(Uint128::zero(), |acc, destination| {
            let allocation_amount =
                checked_mul(total_after_swap_fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            acc.checked_add(allocation_automation_fee).unwrap()
        });

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

    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    let response = after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let swap_fee = custom_fee_percent * receive_amount;
    let total_after_swap_fee = receive_amount - swap_fee;

    let automation_fee = vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .fold(Uint128::zero(), |acc, destination| {
            let allocation_amount =
                checked_mul(total_after_swap_fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            acc.checked_add(allocation_automation_fee).unwrap()
        });

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

    let receive_amount = Uint128::new(234312312);

    SWAP_CACHE
        .save(
            deps.as_mut().storage,
            &SwapCache {
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(receive_amount.into(), vault.get_receive_denom())],
    );

    let response = after_fin_swap(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let config = get_config(&deps.storage).unwrap();
    let swap_fee = min(swap_denom_fee_percent, receive_denom_fee_percent) * receive_amount;
    let total_after_swap_fee = receive_amount - swap_fee;

    let automation_fee = vault
        .destinations
        .iter()
        .filter(|d| d.action == PostExecutionAction::ZDelegate)
        .fold(Uint128::zero(), |acc, destination| {
            let allocation_amount =
                checked_mul(total_after_swap_fee, destination.allocation).unwrap();
            let allocation_automation_fee =
                checked_mul(allocation_amount, config.delegation_fee_percent).unwrap();
            acc.checked_add(allocation_automation_fee).unwrap()
        });

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(swap_fee.into(), vault.get_receive_denom())]
    })));

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: config.fee_collectors[0].address.to_string(),
        amount: vec![Coin::new(automation_fee.into(), vault.get_receive_denom())]
    })));
}
