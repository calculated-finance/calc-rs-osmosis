use crate::{
    constants::TWO_MICRONS,
    contract::{
        AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
    },
    handlers::{
        after_fin_limit_order_retracted::after_fin_limit_order_retracted, get_vault::get_vault,
    },
    helpers::vault_helpers::get_swap_amount,
    state::{
        cache::{LimitOrderCache, LIMIT_ORDER_CACHE},
        triggers::get_trigger,
    },
    tests::{
        helpers::{
            instantiate_contract, setup_active_vault_with_funds, setup_active_vault_with_low_funds,
        },
        mocks::ADMIN,
    },
};
use base::vaults::vault::VaultStatus;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, BankMsg, Coin, CosmosMsg, Decimal256, Reply, SubMsg, SubMsgResponse, SubMsgResult,
    Uint128, WasmMsg,
};
use kujira::fin::ExecuteMsg as FINExecuteMsg;

#[test]
fn with_unfilled_limit_order_should_return_vault_balance_plus_retracted_amount() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount + TWO_MICRONS).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(0, vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: TWO_MICRONS,
                original_offer_amount: TWO_MICRONS,
                filled: Uint128::zero(),
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(
            (vault.balance.amount + TWO_MICRONS).into(),
            "base"
        )]
    })));
}

#[test]
fn with_unfilled_limit_order_and_low_funds_should_return_vault_balance_plus_retracted_amount() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount + TWO_MICRONS).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(0, vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: TWO_MICRONS,
                original_offer_amount: TWO_MICRONS,
                filled: Uint128::zero(),
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(
            (vault.balance.amount + TWO_MICRONS).into(),
            vault.get_swap_denom()
        )]
    })));
}

#[test]
fn with_unfilled_limit_order_should_set_vault_balance_to_zero() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
            Coin::new(0, vault.get_receive_denom()),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.balance.amount, Uint128::zero());
}

#[test]
fn with_new_unfilled_limit_order_should_set_vault_balance_to_zero() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = TWO_MICRONS;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount + TWO_MICRONS).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(0, vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: TWO_MICRONS,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(TWO_MICRONS.into(), vault.get_receive_denom()),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.balance.amount, Uint128::zero());
}

#[test]
fn with_unfilled_limit_order_and_low_funds_should_set_vault_balance_to_zero() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.balance.amount, Uint128::zero());
}

#[test]
fn with_unfilled_limit_order_should_set_vault_status_to_cancelled() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.status, VaultStatus::Cancelled);
}

#[test]
fn with_new_unfilled_limit_order_should_set_vault_status_to_cancelled() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = TWO_MICRONS;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount + TWO_MICRONS).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(0, vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: TWO_MICRONS,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(TWO_MICRONS.into(), vault.get_receive_denom()),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.status, VaultStatus::Cancelled);
}

#[test]
fn with_unfilled_limit_order_and_low_funds_should_set_vault_status_to_cancelled() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(updated_vault.status, VaultStatus::Cancelled);
}

#[test]
fn with_unfilled_limit_order_should_delete_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let trigger = get_trigger(&deps.storage, vault.id).unwrap();

    assert_eq!(trigger, None);
}

#[test]
fn with_new_unfilled_limit_order_should_delete_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = TWO_MICRONS;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount + TWO_MICRONS).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(0, vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: TWO_MICRONS,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(TWO_MICRONS.into(), vault.get_receive_denom()),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let trigger = get_trigger(&deps.storage, vault.id).unwrap();

    assert_eq!(trigger, None);
}

#[test]
fn with_unfilled_limit_order_and_low_funds_should_delete_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    let trigger = get_trigger(&deps.storage, vault.id).unwrap();

    assert_eq!(trigger, None);
}

#[test]
fn with_partially_filled_limit_order_should_return_vault_balance_plus_retracted_amount() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount + Uint128::one()).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                vault.received_amount.amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    let _original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let _swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::one(),
                original_offer_amount: TWO_MICRONS,
                filled: Uint128::one(),
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(
            (vault.balance.amount + Uint128::one()).into(),
            vault.get_swap_denom()
        )]
    })));
}

#[test]
fn with_partially_filled_new_limit_order_should_return_vault_balance_minus_filled_amount() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = TWO_MICRONS / Uint128::new(2);

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount + received_amount).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(0, vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: received_amount,
                original_offer_amount: TWO_MICRONS,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(
            (vault.balance.amount + received_amount).into(),
            vault.get_swap_denom()
        )]
    })));
}

#[test]
fn with_partially_filled_limit_order_and_low_funds_should_return_vault_balance_plus_retracted_amount(
) {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount + Uint128::one()).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                vault.received_amount.amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    let _original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let _swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::one(),
                original_offer_amount: TWO_MICRONS,
                filled: Uint128::one(),
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![Coin::new(
            (vault.balance.amount + Uint128::one()).into(),
            vault.get_swap_denom()
        )]
    })));
}

#[test]
fn with_partially_filled_limit_order_should_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let order_idx = Uint128::new(18);

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount
        / Uint128::new(2);

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount
                    - get_swap_amount(&deps.as_ref(), vault.clone())
                        .unwrap()
                        .amount
                    + received_amount)
                    .into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                vault.received_amount.amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: received_amount,
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
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
fn with_filled_new_limit_order_should_return_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());

    let received_amount = TWO_MICRONS;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(vault.balance.amount.into(), vault.get_swap_denom()),
            Coin::new(received_amount.into(), vault.get_receive_denom()),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount: TWO_MICRONS,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance: vault.balance.clone(),
                receive_denom_balance: Coin::new(0, vault.get_receive_denom()),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
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
    assert_eq!(response.messages.len(), 2);
}

#[test]
fn with_filled_limit_order_should_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_funds(deps.as_mut(), env.clone());
    let order_idx = Uint128::new(18);

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount
                    - get_swap_amount(&deps.as_ref(), vault.clone())
                        .unwrap()
                        .amount)
                    .into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                vault.received_amount.amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
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
fn with_filled_limit_order_and_low_funds_should_return_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new((vault.balance.amount).into(), vault.get_swap_denom()),
            Coin::new(
                vault.received_amount.amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    let _original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let _swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

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
                    (vault.balance.amount).into(),
                    vault.get_swap_denom(),
                ),
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: vault.owner.to_string(),
            amount: vec![Coin::new(
                vault.balance.amount.into(),
                vault.get_swap_denom()
            )],
        }))));
}

#[test]
fn with_filled_limit_order_and_low_funds_should_withdraw_remainder() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());
    let order_idx = Uint128::new(18);

    let received_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount
                    - get_swap_amount(&deps.as_ref(), vault.clone())
                        .unwrap()
                        .amount)
                    .into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                vault.received_amount.amount.into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    let original_offer_amount = get_swap_amount(&deps.as_ref(), vault.clone())
        .unwrap()
        .amount;

    let swap_denom_balance = Coin::new(
        (vault.balance.amount
            - get_swap_amount(&deps.as_ref(), vault.clone())
                .unwrap()
                .amount)
            .into(),
        vault.get_swap_denom(),
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: Uint128::zero(),
                original_offer_amount,
                filled: received_amount,
                quote_price: Decimal256::one(),
                created_at: env.block.time,
                swap_denom_balance,
                receive_denom_balance: vault.received_amount.clone(),
            },
        )
        .unwrap();

    let response = after_fin_limit_order_retracted(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
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
            msg: to_binary(&FINExecuteMsg::WithdrawOrders {
                order_idxs: Some(vec![order_idx]),
            })
            .unwrap(),
            funds: vec![],
        }),
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID
    )));
}
