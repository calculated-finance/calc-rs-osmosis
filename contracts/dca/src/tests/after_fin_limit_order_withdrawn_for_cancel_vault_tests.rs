use base::vaults::vault::VaultStatus;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    BankMsg, Coin, Decimal256, Reply, SubMsg, SubMsgResponse, SubMsgResult, Uint128,
};

use crate::{
    constants::TWO_MICRONS,
    contract::AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
    handlers::{
        after_fin_limit_order_withdrawn_for_cancel_vault::after_fin_limit_order_withdrawn_for_cancel_vault,
        get_vault::get_vault,
    },
    state::{
        cache::{LimitOrderCache, LIMIT_ORDER_CACHE},
        triggers::get_trigger,
    },
    tests::{
        helpers::{instantiate_contract, setup_active_vault_with_low_funds},
        mocks::ADMIN,
    },
};

#[test]
fn with_partially_filled_limit_order_should_return_funds_to_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![
            Coin::new(
                (vault.balance.amount - TWO_MICRONS / Uint128::new(2)).into(),
                vault.get_swap_denom(),
            ),
            Coin::new(
                (TWO_MICRONS / Uint128::new(2)).into(),
                vault.get_receive_denom(),
            ),
        ],
    );

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: TWO_MICRONS / Uint128::new(2),
                original_offer_amount: TWO_MICRONS,
                filled: TWO_MICRONS / Uint128::new(2),
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

    let response = after_fin_limit_order_withdrawn_for_cancel_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
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
            (TWO_MICRONS / Uint128::new(2)).into(),
            vault.get_receive_denom()
        )]
    })));
}

#[test]
fn with_partially_filled_limit_order_should_set_vault_balance_to_zero() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: TWO_MICRONS / Uint128::new(2),
                original_offer_amount: TWO_MICRONS,
                filled: TWO_MICRONS / Uint128::new(2),
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

    after_fin_limit_order_withdrawn_for_cancel_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
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
fn with_partially_filled_limit_order_should_set_vault_status_to_cancelled() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: TWO_MICRONS / Uint128::new(2),
                original_offer_amount: TWO_MICRONS,
                filled: TWO_MICRONS / Uint128::new(2),
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

    after_fin_limit_order_withdrawn_for_cancel_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
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
fn with_partially_filled_limit_order_should_delete_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &vec![]));

    let vault = setup_active_vault_with_low_funds(deps.as_mut(), env.clone());

    LIMIT_ORDER_CACHE
        .save(
            deps.as_mut().storage,
            &LimitOrderCache {
                order_idx: Uint128::new(18),
                offer_amount: TWO_MICRONS / Uint128::new(2),
                original_offer_amount: TWO_MICRONS,
                filled: TWO_MICRONS / Uint128::new(2),
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

    after_fin_limit_order_withdrawn_for_cancel_vault(
        deps.as_mut(),
        env,
        Reply {
            id: AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
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
