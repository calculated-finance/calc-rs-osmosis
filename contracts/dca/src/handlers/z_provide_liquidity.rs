use crate::{
    constants::{AFTER_BOND_LP_TOKENS_REPLY_ID, AFTER_PROVIDE_LIQUIDITY_REPLY_ID},
    error::ContractError,
    helpers::{authz::create_authz_exec_message, validation::assert_exactly_one_asset},
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

#[cfg(test)]
mod z_provide_liquidity_tests {
    use super::*;
    use crate::{
        constants::ONE,
        handlers::z_provide_liquidity::{bond_lp_tokens, log_bond_lp_tokens_result},
        helpers::authz::create_authz_exec_message,
        state::cache::{ProvideLiquidityCache, PROVIDE_LIQUIDITY_CACHE},
        tests::mocks::{calc_mock_dependencies, DENOM_STAKE, DENOM_UOSMO, USER},
        types::post_execution_action::LockableDuration,
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Attribute, BankMsg, Coin, Decimal, Reply, SubMsg, SubMsgResponse, Uint128,
    };
    use osmosis_std::types::osmosis::{
        gamm::v1beta1::MsgJoinSwapExternAmountIn, lockup::MsgLockTokens,
    };

    #[test]
    fn with_no_asset_fails() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(&env.contract.address.to_string(), &[]);

        let response = z_provide_liquidity_handler(
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

        let response = z_provide_liquidity_handler(
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

        z_provide_liquidity_handler(
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

        let response = z_provide_liquidity_handler(
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

        let response = z_provide_liquidity_handler(
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
    fn sends_the_lp_tokens_to_the_provider_address() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let pool_id = 1;
        let provider_address = Addr::unchecked(USER);

        let lp_tokens_minted = Coin::new(100000, format!("gamm/pool/{}", pool_id));

        deps.querier
            .update_balance(env.contract.address.clone(), vec![lp_tokens_minted.clone()]);

        PROVIDE_LIQUIDITY_CACHE
            .save(
                deps.as_mut().storage,
                &ProvideLiquidityCache {
                    provider_address: provider_address.clone(),
                    pool_id,
                    duration: LockableDuration::OneDay,
                },
            )
            .unwrap();

        let response = bond_lp_tokens(deps.as_ref(), env.clone()).unwrap();

        assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
            to_address: provider_address.to_string(),
            amount: vec![lp_tokens_minted],
        })));
    }

    #[test]
    fn bonds_the_lp_tokens_from_the_provider_address() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let pool_id = 1;
        let provider_address = Addr::unchecked(USER);

        let lp_tokens_minted = Coin::new(100000, format!("gamm/pool/{}", pool_id));

        deps.querier
            .update_balance(env.contract.address.clone(), vec![lp_tokens_minted.clone()]);

        let cache = ProvideLiquidityCache {
            provider_address: provider_address.clone(),
            pool_id,
            duration: LockableDuration::OneDay,
        };

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
                    coins: vec![lp_tokens_minted.into()],
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
}
