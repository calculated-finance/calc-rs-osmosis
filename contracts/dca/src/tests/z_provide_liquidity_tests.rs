use super::mocks::USER;
use crate::{
    constants::ONE,
    contract::{
        AFTER_BOND_LP_TOKENS_REPLY_ID, AFTER_PROVIDE_LIQUIDITY_REPLY_ID,
        AFTER_SEND_LP_TOKENS_REPLY_ID,
    },
    handlers::z_provide_liquidity::{
        bond_lp_tokens, log_bond_lp_tokens_result, provide_liquidity_handler, send_lp_tokens,
    },
    helpers::authz_helpers::create_authz_exec_message,
    state::cache::{ProvideLiquidityCache, PROVIDE_LIQUIDITY_CACHE},
    tests::mocks::{calc_mock_dependencies, DENOM_STAKE, DENOM_UOSMO},
    types::post_execution_action::LockableDuration,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Attribute, BankMsg, Coin, CosmosMsg, Decimal, Reply, SubMsg, SubMsgResponse, Uint128,
};
use osmosis_std::types::osmosis::{
    gamm::v1beta1::MsgJoinSwapExternAmountIn, lockup::MsgLockTokens,
};

#[test]
fn with_no_asset_fails() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(&env.contract.address.to_string(), &[]);

    let response = provide_liquidity_handler(
        deps.as_mut(),
        env,
        info.clone(),
        Addr::unchecked(USER),
        1,
        LockableDuration::OneDay,
        None,
    )
    .unwrap_err();

    assert_eq!(
        response.to_string(),
        "Error: received 0 denoms but required exactly 1",
    );
}

#[test]
fn with_more_than_one_asset_fails() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(
        &env.contract.address.to_string(),
        &[Coin::new(100, DENOM_STAKE), Coin::new(100, DENOM_UOSMO)],
    );

    let response = provide_liquidity_handler(
        deps.as_mut(),
        env,
        info.clone(),
        Addr::unchecked(USER),
        1,
        LockableDuration::OneDay,
        None,
    )
    .unwrap_err();

    assert_eq!(
        response.to_string(),
        "Error: received 2 denoms but required exactly 1",
    );
}

#[test]
fn updates_the_cache_before_providing_liquidity() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(
        &env.contract.address.to_string(),
        &[Coin::new(100, DENOM_STAKE)],
    );

    let pool_id = 1;
    let provider_address = Addr::unchecked(USER);
    let duration = LockableDuration::OneDay;

    provide_liquidity_handler(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        provider_address.clone(),
        pool_id,
        duration.clone(),
        None,
    )
    .unwrap();

    let cache = PROVIDE_LIQUIDITY_CACHE.load(deps.as_ref().storage).unwrap();

    assert_eq!(
        cache,
        ProvideLiquidityCache {
            provider_address,
            pool_id,
            duration,
            lp_token_balance: None
        }
    );
}

#[test]
fn sends_provide_liquidity_message() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(
        &env.contract.address.to_string(),
        &[Coin::new(100, DENOM_STAKE)],
    );

    let pool_id = 1;

    let response = provide_liquidity_handler(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        Addr::unchecked(USER),
        pool_id,
        LockableDuration::OneDay,
        None,
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        MsgJoinSwapExternAmountIn {
            sender: env.contract.address.to_string(),
            pool_id,
            token_in: Some(info.funds[0].clone().into()),
            share_out_min_amount: Uint128::one().to_string(),
        },
        AFTER_PROVIDE_LIQUIDITY_REPLY_ID,
    )));
}

#[test]
fn sends_provide_liquidity_message_with_slippage_included() {
    let mut deps = calc_mock_dependencies();
    let env = mock_env();
    let info = mock_info(
        &env.contract.address.to_string(),
        &[Coin::new(100, DENOM_STAKE)],
    );

    let pool_id = 1;
    let slippage_tolerance = Decimal::percent(10);

    let response = provide_liquidity_handler(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        Addr::unchecked(USER),
        pool_id,
        LockableDuration::OneDay,
        Some(slippage_tolerance),
    )
    .unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        MsgJoinSwapExternAmountIn {
            sender: env.contract.address.to_string(),
            pool_id,
            token_in: Some(info.funds[0].clone().into()),
            share_out_min_amount: (ONE * (Decimal::one() - slippage_tolerance)).to_string(),
        },
        AFTER_PROVIDE_LIQUIDITY_REPLY_ID,
    )));
}

#[test]
fn updates_the_cache_before_sending_lp_tokens() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let pool_id = 1;

    let cache_before = ProvideLiquidityCache {
        provider_address: Addr::unchecked(USER),
        pool_id,
        duration: LockableDuration::OneDay,
        lp_token_balance: None,
    };

    let lp_tokens_minted = Coin::new(100000, format!("gamm/pool/{}", pool_id));

    deps.querier
        .update_balance(env.contract.address.clone(), vec![lp_tokens_minted.clone()]);

    PROVIDE_LIQUIDITY_CACHE
        .save(deps.as_mut().storage, &cache_before)
        .unwrap();

    send_lp_tokens(deps.as_mut(), env.clone()).unwrap();

    let cache_after = PROVIDE_LIQUIDITY_CACHE.load(deps.as_ref().storage).unwrap();

    assert_eq!(
        cache_after,
        ProvideLiquidityCache {
            lp_token_balance: Some(lp_tokens_minted),
            ..cache_before
        }
    );
}

#[test]
fn sends_the_lp_tokens_to_the_provider_address() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let pool_id = 1;
    let provider_address = Addr::unchecked(USER);

    let cache_before = ProvideLiquidityCache {
        provider_address: provider_address.clone(),
        pool_id,
        duration: LockableDuration::OneDay,
        lp_token_balance: None,
    };

    let lp_tokens_minted = Coin::new(100000, format!("gamm/pool/{}", pool_id));

    deps.querier
        .update_balance(env.contract.address.clone(), vec![lp_tokens_minted.clone()]);

    PROVIDE_LIQUIDITY_CACHE
        .save(deps.as_mut().storage, &cache_before)
        .unwrap();

    let response = send_lp_tokens(deps.as_mut(), env.clone()).unwrap();

    assert!(response.messages.contains(&SubMsg::reply_on_success(
        CosmosMsg::Bank(BankMsg::Send {
            to_address: provider_address.to_string(),
            amount: vec![lp_tokens_minted],
        }),
        AFTER_SEND_LP_TOKENS_REPLY_ID,
    )));
}

#[test]
fn bonds_the_lp_tokens_from_the_provider_address() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let pool_id = 1;
    let provider_address = Addr::unchecked(USER);
    let duration = LockableDuration::OneDay;
    let lp_token_balance = Coin::new(100000, format!("gamm/pool/{}", pool_id));

    let cache = ProvideLiquidityCache {
        provider_address: provider_address.clone(),
        pool_id,
        duration,
        lp_token_balance: Some(lp_token_balance.clone()),
    };

    deps.querier
        .update_balance(provider_address.clone(), vec![lp_token_balance.clone()]);

    PROVIDE_LIQUIDITY_CACHE
        .save(deps.as_mut().storage, &cache)
        .unwrap();

    let response = bond_lp_tokens(deps.as_ref(), env.clone()).unwrap();

    assert!(response.messages.contains(&SubMsg::reply_always(
        create_authz_exec_message(
            env.contract.address,
            "/osmosis.lockup.MsgLockTokens".to_string(),
            MsgLockTokens {
                owner: provider_address.to_string(),
                duration: Some(cache.duration.into()),
                coins: vec![cache.lp_token_balance.unwrap().into()],
            },
        ),
        AFTER_BOND_LP_TOKENS_REPLY_ID,
    )))
}

#[test]
fn logs_the_bond_lp_tokens_result_on_success() {
    let mut deps = mock_dependencies();

    let response = log_bond_lp_tokens_result(
        deps.as_mut(),
        Reply {
            id: AFTER_BOND_LP_TOKENS_REPLY_ID,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: None,
            }),
        },
    )
    .unwrap();

    assert!(response
        .attributes
        .contains(&Attribute::new("bond_lp_tokens_result", "success")));
}

#[test]
fn logs_the_bond_lp_tokens_result_on_failure() {
    let mut deps = mock_dependencies();

    let response = log_bond_lp_tokens_result(
        deps.as_mut(),
        Reply {
            id: AFTER_BOND_LP_TOKENS_REPLY_ID,
            result: cosmwasm_std::SubMsgResult::Err("error code 4".to_string()),
        },
    )
    .unwrap();

    assert!(response
        .attributes
        .contains(&Attribute::new("bond_lp_tokens_result", "failure")));
}
