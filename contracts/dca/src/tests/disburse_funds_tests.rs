use crate::{
    constants::{ONE, TEN, TWO_MICRONS},
    contract::AFTER_SWAP_REPLY_ID,
    handlers::{
        disburse_funds::disburse_funds, get_events_by_resource_id::get_events_by_resource_id,
    },
    helpers::vault_helpers::get_swap_amount,
    state::{
        cache::{SwapCache, SWAP_CACHE},
        config::{create_custom_fee, get_config, FeeCollector},
        swap_adjustments::update_swap_adjustments,
        vaults::get_vault,
    },
    tests::{
        helpers::{
            instantiate_contract, instantiate_contract_with_multiple_fee_collectors,
            setup_new_vault,
        },
        mocks::{ADMIN, DENOM_UOSMO},
    },
    types::{
        dca_plus_config::DcaPlusConfig,
        event::{Event, EventBuilder, EventData, ExecutionSkippedReason},
        position_type::PositionType,
        post_execution_action::PostExecutionAction,
        vault::{Vault, VaultStatus},
    },
};
use base::helpers::{coin_helpers::add_to_coin, math_helpers::checked_mul};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, Coin, Decimal, Reply, SubMsg, SubMsgResponse, SubMsgResult, Uint128,
};
use std::{cmp::min, str::FromStr};

#[test]
fn with_succcesful_swap_returns_funds_to_destination() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());
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

    let response = disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.destinations.first().unwrap().address.to_string(),
        amount: vec![Coin::new(
            (receive_amount - fee - automation_fees.amount).into(),
            vault.get_receive_denom(),
        )],
    },)));
}

#[test]
fn with_succcesful_swap_returns_fee_to_fee_collector() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());
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

    let response = disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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

    instantiate_contract_with_multiple_fee_collectors(
        deps.as_mut(),
        env.clone(),
        mock_info(ADMIN, &vec![]),
        vec![
            FeeCollector {
                address: "fee_collector_1".to_string(),
                allocation: Decimal::percent(20),
            },
            FeeCollector {
                address: "fee_collector_2".to_string(),
                allocation: Decimal::percent(80),
            },
        ],
    );

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());
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

    let response = disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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

    for fee_collector in config.fee_collectors.iter() {
        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: fee_collector.address.to_string(),
            amount: vec![Coin::new(
                checked_mul(swap_fee, fee_collector.allocation)
                    .unwrap()
                    .into(),
                vault.get_receive_denom()
            )]
        })));

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: fee_collector.address.to_string(),
            amount: vec![Coin::new(
                checked_mul(automation_fee, fee_collector.allocation)
                    .unwrap()
                    .into(),
                vault.get_receive_denom()
            )]
        })));
    }
}

#[test]
fn with_succcesful_swap_adjusts_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());
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

    disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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
            - get_swap_amount(&deps.as_ref(), &env, &vault)
                .unwrap()
                .amount
    );
}

#[test]
fn with_succcesful_swap_adjusts_swapped_amount_stat() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());
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
                    - get_swap_amount(&deps.as_ref(), &env, &vault)
                        .unwrap()
                        .amount)
                    .into(),
                vault.get_swap_denom(),
            ),
            Coin::new(receive_amount.into(), vault.get_receive_denom()),
        ],
    );

    disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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
        get_swap_amount(&deps.as_ref(), &env, &vault)
            .unwrap()
            .amount
    );
}

#[test]
fn with_succcesful_swap_adjusts_received_amount_stat() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());
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

    disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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
fn with_succcesful_swap_with_dca_plus_escrows_funds() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

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

    update_swap_adjustments(
        deps.as_mut().storage,
        PositionType::Exit,
        vec![
            (30, Decimal::from_str("1.0").unwrap()),
            (35, Decimal::from_str("1.0").unwrap()),
            (40, Decimal::from_str("1.0").unwrap()),
            (45, Decimal::from_str("1.0").unwrap()),
            (50, Decimal::from_str("1.0").unwrap()),
            (55, Decimal::from_str("1.0").unwrap()),
            (60, Decimal::from_str("1.0").unwrap()),
            (70, Decimal::from_str("1.0").unwrap()),
            (80, Decimal::from_str("1.0").unwrap()),
            (90, Decimal::from_str("1.0").unwrap()),
        ],
        env.block.time,
    )
    .unwrap();

    let response = disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

    let escrow_level = updated_vault.dca_plus_config.clone().unwrap().escrow_level;
    let escrow_amount = escrow_level * receive_amount;

    assert_eq!(
        escrow_amount,
        updated_vault
            .dca_plus_config
            .clone()
            .unwrap()
            .escrowed_balance
            .amount
    );
    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: updated_vault
            .destinations
            .first()
            .unwrap()
            .address
            .to_string(),
        amount: vec![Coin::new(
            (receive_amount - escrow_amount).into(),
            updated_vault.get_receive_denom(),
        )],
    },)));
    assert_ne!(escrow_level, Decimal::zero());
    assert_ne!(escrow_amount, Uint128::zero());
}

#[test]
fn with_succcesful_swap_publishes_dca_execution_completed_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

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

    update_swap_adjustments(
        deps.as_mut().storage,
        PositionType::Exit,
        vec![
            (30, Decimal::from_str("1.0").unwrap()),
            (35, Decimal::from_str("1.0").unwrap()),
            (40, Decimal::from_str("1.0").unwrap()),
            (45, Decimal::from_str("1.0").unwrap()),
            (50, Decimal::from_str("1.0").unwrap()),
            (55, Decimal::from_str("1.0").unwrap()),
            (60, Decimal::from_str("1.0").unwrap()),
            (70, Decimal::from_str("1.0").unwrap()),
            (80, Decimal::from_str("1.0").unwrap()),
            (90, Decimal::from_str("1.0").unwrap()),
        ],
        env.block.time,
    )
    .unwrap();

    disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    let config = get_config(deps.as_ref().storage).unwrap();

    let inverted_fee_rate =
        Decimal::one() - (config.swap_fee_percent + config.delegation_fee_percent);
    let received_amount =
        updated_vault.received_amount.amount * (Decimal::one() / inverted_fee_rate);
    let fee = received_amount - updated_vault.received_amount.amount - Uint128::new(2); // rounding

    assert!(events.contains(&Event {
        id: 1,
        resource_id: vault.id,
        timestamp: env.block.time,
        block_height: env.block.height,
        data: EventData::DcaVaultExecutionCompleted {
            sent: updated_vault.swapped_amount,
            received: add_to_coin(updated_vault.received_amount, fee),
            fee: Coin::new(fee.into(), vault.get_receive_denom())
        }
    }))
}

#[test]
fn with_succcesful_swap_with_dca_plus_publishes_execution_completed_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

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

    update_swap_adjustments(
        deps.as_mut().storage,
        PositionType::Exit,
        vec![
            (30, Decimal::from_str("1.0").unwrap()),
            (35, Decimal::from_str("1.0").unwrap()),
            (40, Decimal::from_str("1.0").unwrap()),
            (45, Decimal::from_str("1.0").unwrap()),
            (50, Decimal::from_str("1.0").unwrap()),
            (55, Decimal::from_str("1.0").unwrap()),
            (60, Decimal::from_str("1.0").unwrap()),
            (70, Decimal::from_str("1.0").unwrap()),
            (80, Decimal::from_str("1.0").unwrap()),
            (90, Decimal::from_str("1.0").unwrap()),
        ],
        env.block.time,
    )
    .unwrap();

    disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(&deps.storage, vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(&Event {
        id: 1,
        resource_id: vault.id,
        timestamp: env.block.time,
        block_height: env.block.height,
        data: EventData::DcaVaultExecutionCompleted {
            sent: updated_vault.swapped_amount,
            received: updated_vault.received_amount,
            fee: Coin::new(0, vault.get_receive_denom())
        }
    }))
}

#[test]
fn with_failed_swap_and_insufficient_funds_does_not_reduce_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let balance = Coin::new(TWO_MICRONS.into(), DENOM_UOSMO);

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            balance: balance.clone(),
            ..Vault::default()
        },
    );

    let reply = Reply {
        id: AFTER_SWAP_REPLY_ID,
        result: SubMsgResult::Err("Generic failure".to_string()),
    };

    disburse_funds(deps.as_mut(), &env, reply).unwrap();

    let updated_vault = get_vault(&mut deps.storage, vault.id).unwrap();

    assert_eq!(vault.balance, balance);
    assert_eq!(updated_vault.balance, balance);
}

#[test]
fn with_failed_swap_publishes_skipped_event_with_slippage_failure() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    let reply = Reply {
        id: AFTER_SWAP_REPLY_ID,
        result: SubMsgResult::Err("failed for slippage".to_string()),
    };

    disburse_funds(deps.as_mut(), &env, reply).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(
        &EventBuilder::new(
            vault.id,
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

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    let reply = Reply {
        id: AFTER_SWAP_REPLY_ID,
        result: SubMsgResult::Err("failed for slippage".to_string()),
    };

    disburse_funds(deps.as_mut(), &env, reply).unwrap();

    let vault = get_vault(&mut deps.storage, vault.id).unwrap();

    assert_eq!(vault.status, VaultStatus::Active);
}

#[test]
fn with_failed_swap_does_not_reduce_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    let reply = Reply {
        id: AFTER_SWAP_REPLY_ID,
        result: SubMsgResult::Err("failed for slippage".to_string()),
    };

    disburse_funds(deps.as_mut(), &env, reply).unwrap();

    let vault = get_vault(&mut deps.storage, vault.id).unwrap();

    assert_eq!(vault.balance, Coin::new(TEN.into(), vault.get_swap_denom()));
}

#[test]
fn with_custom_fee_for_base_denom_takes_custom_fee() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

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

    let response = disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

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

    let response = disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

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

    let response = disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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

#[test]
fn with_insufficient_remaining_funds_sets_vault_to_inactive() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            balance: Coin::new(ONE.into(), DENOM_UOSMO),
            swap_amount: ONE,
            ..Vault::default()
        },
    );

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
        vec![Coin::new(1000000, vault.get_receive_denom())],
    );

    disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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
fn for_dca_plus_vault_with_failed_swap_publishes_slippage_tolerance_exceeded_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            balance: Coin::new(ONE.into(), DENOM_UOSMO),
            swap_amount: ONE,
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

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
        vec![Coin::new(1000000, vault.get_receive_denom())],
    );

    disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
            result: SubMsgResult::Err("slippage exceeded".to_string()),
        },
    )
    .unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(&Event {
        id: 1,
        resource_id: vault.id,
        timestamp: env.block.time,
        block_height: env.block.height,
        data: EventData::DcaVaultExecutionSkipped {
            reason: ExecutionSkippedReason::SlippageToleranceExceeded
        }
    }));
}

#[test]
fn for_dca_plus_vault_with_insufficient_remaining_funds_sets_vault_to_inactive() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            balance: Coin::new(49999, DENOM_UOSMO),
            swap_amount: ONE,
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

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
        vec![Coin::new(1000000, vault.get_receive_denom())],
    );

    disburse_funds(
        deps.as_mut(),
        &env,
        Reply {
            id: AFTER_SWAP_REPLY_ID,
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
