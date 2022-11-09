use crate::{
    contract::{
        AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
    },
    handlers::{
        after_fin_limit_order_retracted::after_fin_limit_order_retracted, get_vault::get_vault,
    },
    state::{
        cache::{LimitOrderCache, LIMIT_ORDER_CACHE},
        triggers::get_trigger,
    },
    tests::helpers::{setup_active_vault_with_funds, setup_active_vault_with_low_funds},
};
use base::vaults::vault::VaultStatus;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    to_binary, BankMsg, Coin, CosmosMsg, Event, Reply, SubMsg, SubMsgResponse, SubMsgResult,
    Uint128, WasmMsg,
};
use kujira::fin::ExecuteMsg as FINExecuteMsg;

#[test]
fn with_unfilled_limit_order_should_return_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

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

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

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
        amount: vec![Coin::new(
            vault.balance.amount.into(),
            vault.get_swap_denom()
        )]
    })));
}

#[test]
fn with_unfilled_limit_order_and_low_funds_should_set_vault_balance_to_zero() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

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

    after_fin_limit_order_retracted(
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

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.balance.amount, Uint128::zero());
}

#[test]
fn with_unfilled_limit_order_and_low_funds_should_set_vault_status_to_cancelled() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

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

    after_fin_limit_order_retracted(
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

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.status, VaultStatus::Cancelled);
}

#[test]
fn with_unfilled_limit_order_and_low_funds_should_delete_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

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

    after_fin_limit_order_retracted(
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

    let trigger = get_trigger(&deps.storage, vault.id).unwrap();

    assert_eq!(trigger, None);
}

#[test]
fn with_partially_filled_limit_order_should_return_vault_balance_minus_filled_amount() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

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
            vault.get_swap_denom()
        )]
    })));
}

#[test]
fn with_partially_filled_limit_order_should_return_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
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

    assert!(response.messages.contains(&SubMsg::reply_always(
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

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

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
            vault.get_swap_denom()
        )]
    })));
}

#[test]
fn with_partially_filled_limit_order_and_low_funds_should_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());
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

    assert!(response.messages.contains(&SubMsg::reply_always(
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
            vault.get_swap_denom()
        )]
    })));
    assert_eq!(response.messages.len(), 2);
}

#[test]
fn with_filled_limit_order_should_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
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

    assert!(response.messages.contains(&SubMsg::reply_always(
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

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());
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

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());
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

    assert!(response.messages.contains(&SubMsg::reply_always(
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
