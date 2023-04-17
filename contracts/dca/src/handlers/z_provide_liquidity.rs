use crate::{
    contract::{AFTER_BOND_LP_TOKENS_REPLY_ID, AFTER_PROVIDE_LIQUIDITY_REPLY_ID},
    error::ContractError,
    helpers::{
        authz_helpers::create_authz_exec_message, validation_helpers::assert_exactly_one_asset,
    },
    state::cache::{ProvideLiquidityCache, PROVIDE_LIQUIDITY_CACHE},
    types::post_execution_action::LockableDuration,
};
use cosmwasm_std::{
    Addr, BankMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply, Response, SubMsg, SubMsgResult,
    Uint128,
};
use osmosis_std::types::osmosis::{
    gamm::v1beta1::{MsgJoinSwapExternAmountIn, QueryCalcJoinPoolSharesRequest},
    lockup::MsgLockTokens,
};

pub fn z_provide_liquidity_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    provider_address: Addr,
    pool_id: u64,
    duration: LockableDuration,
    slippage_tolerance: Option<Decimal>,
) -> Result<Response, ContractError> {
    assert_exactly_one_asset(info.funds.clone())?;

    PROVIDE_LIQUIDITY_CACHE.save(
        deps.storage,
        &ProvideLiquidityCache {
            provider_address,
            pool_id,
            duration,
        },
    )?;

    let share_out_min_amount = String::from(slippage_tolerance.map_or(
        Uint128::one(),
        |slippage_tolerance| {
            QueryCalcJoinPoolSharesRequest {
                pool_id,
                tokens_in: vec![info.funds[0].clone().into()],
            }
            .query(&deps.querier)
            .expect("share amount out response")
            .share_out_amount
            .parse::<Uint128>()
            .expect("share amount out value")
                * (Decimal::one() - slippage_tolerance)
        },
    ));

    Ok(Response::new()
        .add_attributes(vec![
            ("lp_pool_id", pool_id.clone().to_string()),
            ("lp_share_out_min_amount", share_out_min_amount.clone()),
        ])
        .add_submessage(SubMsg::reply_on_success(
            MsgJoinSwapExternAmountIn {
                sender: env.contract.address.to_string(),
                pool_id,
                token_in: Some(info.funds[0].clone().into()),
                share_out_min_amount,
            },
            AFTER_PROVIDE_LIQUIDITY_REPLY_ID,
        )))
}

pub fn bond_lp_tokens(deps: Deps, env: Env) -> Result<Response, ContractError> {
    let cache = PROVIDE_LIQUIDITY_CACHE.load(deps.storage)?;

    let lp_token_balance = deps.querier.query_balance(
        &env.contract.address,
        format!("gamm/pool/{}", cache.pool_id),
    )?;

    Ok(Response::new()
        .add_attributes(vec![
            (
                "bond_lp_tokens_amount",
                lp_token_balance.clone().to_string(),
            ),
            (
                "bond_lp_tokens_duration",
                cache.duration.clone().to_string(),
            ),
        ])
        .add_submessages(vec![
            SubMsg::new(BankMsg::Send {
                to_address: cache.provider_address.to_string(),
                amount: vec![lp_token_balance.clone()],
            }),
            SubMsg::reply_always(
                create_authz_exec_message(
                    env.contract.address,
                    "/osmosis.lockup.MsgLockTokens".to_string(),
                    MsgLockTokens {
                        owner: cache.provider_address.to_string(),
                        duration: Some(cache.duration.into()),
                        coins: vec![lp_token_balance.into()],
                    },
                ),
                AFTER_BOND_LP_TOKENS_REPLY_ID,
            ),
        ]))
}

pub fn log_bond_lp_tokens_result(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    PROVIDE_LIQUIDITY_CACHE.remove(deps.storage);

    let result = match reply.result {
        SubMsgResult::Ok(_) => "success".to_string(),
        SubMsgResult::Err(_) => "failure".to_string(),
    };

    Ok(Response::new().add_attribute("bond_lp_tokens_result", result))
}
