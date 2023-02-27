use crate::{
    constants::TWO_MICRONS,
    contract::{
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID, AFTER_FIN_SWAP_REPLY_ID,
    },
    handlers::after_fin_limit_order_withdrawn_for_execute_trigger::after_fin_limit_order_withdrawn_for_execute_vault,
    helpers::vault_helpers::get_swap_amount,
    state::cache::{LimitOrderCache, LIMIT_ORDER_CACHE},
    tests::{
        helpers::{instantiate_contract, setup_active_vault_with_funds},
        mocks::ADMIN,
    },
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, BankMsg, Coin, CosmosMsg, Decimal256, Reply, SubMsg, SubMsgResponse, SubMsgResult,
    Uint128, WasmMsg,
};
use kujira::fin::ExecuteMsg as FINExecuteMsg;

#[test]
fn after_succcesful_withdrawal_of_limit_order_invokes_a_fin_swap() {
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
            funds: vec![get_swap_amount(&deps.as_ref(), vault.clone()).unwrap()]
        }),
        AFTER_FIN_SWAP_REPLY_ID
    )));
}

#[test]
fn after_succcesful_withdrawal_of_limit_order_returns_filled_amount_to_owner() {
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

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: vault.owner.to_string(),
            amount: vec![Coin::new(TWO_MICRONS.into(), vault.get_receive_denom())]
        }))));
}
