use crate::{
    contract::{
        AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
    },
    handlers::after_fin_limit_order_retracted::after_fin_limit_order_retracted,
    state::{
        cache::{Cache, LimitOrderCache, CACHE, LIMIT_ORDER_CACHE},
        pairs::PAIRS,
        triggers::save_trigger,
        vaults::save_vault,
    },
    vault::{Vault, VaultBuilder},
};
use base::{
    pair::Pair,
    triggers::trigger::{TimeInterval, Trigger, TriggerConfiguration},
    vaults::vault::VaultStatus,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, Event, Reply, SubMsg, SubMsgResponse,
    SubMsgResult, Uint128, WasmMsg,
};
use kujira::fin::ExecuteMsg as FINExecuteMsg;

fn setup_vault_with_funds(deps: DepsMut, env: Env) -> Vault {
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

    vault
}

fn setup_vault_with_low_funds(deps: DepsMut, env: Env) -> Vault {
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

    vault
}

#[test]
fn with_unfilled_limit_order_should_return_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: vault.get_swap_amount().amount,
                original_offer_amount: vault.get_swap_amount().amount,
                filled: Uint128::zero(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![
                    Event::new("wasm").add_attribute("amount", vault.get_swap_amount().amount)
                ],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(vault.balance.amount.into(), "base")]
    })));
}

#[test]
fn with_unfilled_limit_order_and_low_funds_should_return_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_low_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: vault.get_swap_amount().amount,
                original_offer_amount: vault.get_swap_amount().amount,
                filled: Uint128::zero(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![
                    Event::new("wasm").add_attribute("amount", vault.get_swap_amount().amount)
                ],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(vault.balance.amount.into(), "base")]
    })));
}

#[test]
fn with_partially_filled_limit_order_should_return_vault_balance_minus_filled_amount() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: vault.get_swap_amount().amount / Uint128::new(2),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount / Uint128::new(2),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm")
                    .add_attribute("amount", vault.get_swap_amount().amount / Uint128::new(2))],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(
            (vault.balance.amount - vault.get_swap_amount().amount / Uint128::new(2)).into(),
            "base"
        )]
    })));
}

#[test]
fn with_partially_filled_limit_order_should_return_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_funds(deps.as_mut(), env.clone());
    let order_idx = Uint128::new(18);

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx,
                offer_amount: vault.get_swap_amount().amount / Uint128::new(2),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount / Uint128::new(2),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm")
                    .add_attribute("amount", vault.get_swap_amount().amount / Uint128::new(2))],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: vault.pair.address.to_string(),
            msg: to_binary(&FINExecuteMsg::WithdrawOrders {
                order_idxs: Some(vec![order_idx]),
            })
            .unwrap(),
            funds: vec![],
        }),
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID
    )));
}

#[test]
fn with_partially_filled_limit_order_and_low_funds_should_return_vault_balance_minus_filled_amount()
{
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_low_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: vault.get_swap_amount().amount / Uint128::new(2),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount / Uint128::new(2),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm")
                    .add_attribute("amount", vault.get_swap_amount().amount / Uint128::new(2))],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(
            (vault.balance.amount - vault.get_swap_amount().amount / Uint128::new(2)).into(),
            "base"
        )]
    })));
}

#[test]
fn with_partially_filled_limit_order_and_low_funds_should_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_low_funds(deps.as_mut(), env.clone());
    let order_idx = Uint128::new(18);

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx,
                offer_amount: vault.get_swap_amount().amount / Uint128::new(2),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount / Uint128::new(2),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm")
                    .add_attribute("amount", vault.get_swap_amount().amount / Uint128::new(2))],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: vault.pair.address.to_string(),
            msg: to_binary(&FINExecuteMsg::WithdrawOrders {
                order_idxs: Some(vec![order_idx]),
            })
            .unwrap(),
            funds: vec![],
        }),
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID
    )));
}

#[test]
fn with_filled_limit_order_should_return_vault_balance_minus_swap_amount() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_funds(deps.as_mut(), env.clone());

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

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm").add_attribute("amount", Uint128::zero())],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(
            (vault.balance.amount - vault.get_swap_amount().amount).into(),
            "base"
        )]
    })));
    assert_eq!(response.messages.len(), 2);
}

#[test]
fn with_filled_limit_order_should_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_funds(deps.as_mut(), env.clone());
    let order_idx = Uint128::new(18);

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx,
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm").add_attribute("amount", Uint128::zero())],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: vault.pair.address.to_string(),
            msg: to_binary(&FINExecuteMsg::WithdrawOrders {
                order_idxs: Some(vec![order_idx]),
            })
            .unwrap(),
            funds: vec![],
        }),
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID
    )));
}

#[test]
fn with_filled_limit_order_and_low_funds_should_return_no_funds() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_low_funds(deps.as_mut(), env.clone());
    let order_idx = Uint128::new(18);

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx,
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm").add_attribute("amount", Uint128::zero())],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.iter().all(|msg| {
        match msg.msg {
            CosmosMsg::Bank(BankMsg::Send { .. }) => false,
            _ => true,
        }
    }));
}

#[test]
fn with_filled_limit_order_and_low_funds_should_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_vault_with_low_funds(deps.as_mut(), env.clone());
    let order_idx = Uint128::new(18);

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx,
                offer_amount: Uint128::zero(),
                original_offer_amount: vault.get_swap_amount().amount,
                filled: vault.get_swap_amount().amount,
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![Event::new("wasm").add_attribute("amount", Uint128::zero())],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: vault.pair.address.to_string(),
            msg: to_binary(&FINExecuteMsg::WithdrawOrders {
                order_idxs: Some(vec![order_idx]),
            })
            .unwrap(),
            funds: vec![],
        }),
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID
    )));
}
