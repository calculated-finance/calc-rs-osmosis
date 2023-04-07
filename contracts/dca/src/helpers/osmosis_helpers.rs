use crate::{
    constants::OSMOSIS_SWAP_FEE_RATE,
    types::{pair::Pair, position_type::PositionType},
};
use cosmwasm_std::{
    Addr, Coin, Decimal, Env, QuerierWrapper, ReplyOn, StdError, StdResult, SubMsg, Uint128,
};
use osmosis_std::{
    shim::Duration,
    types::osmosis::{
        gamm::{v1beta1::MsgJoinSwapExternAmountIn, v2::QuerySpotPriceRequest},
        lockup::MsgLockTokens,
        poolmanager::v1beta1::{MsgSwapExactAmountIn, PoolmanagerQuerier, SwapAmountInRoute},
    },
};
use std::str::FromStr;

pub fn query_belief_price(
    querier: &QuerierWrapper,
    pair: &Pair,
    swap_denom: &str,
) -> StdResult<Decimal> {
    if ![pair.base_denom.clone(), pair.quote_denom.clone()].contains(&swap_denom.to_string()) {
        return Err(StdError::generic_err(format!(
            "Provided swap denom {} not in pair {}",
            swap_denom, pair.pool_id
        )));
    }
    let position_type = match swap_denom == pair.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let (base_asset_denom, quote_asset_denom) = match position_type {
        PositionType::Enter => (pair.base_denom.clone(), pair.quote_denom.clone()),
        PositionType::Exit => (pair.quote_denom.clone(), pair.base_denom.clone()),
    };

    QuerySpotPriceRequest {
        pool_id: pair.pool_id,
        base_asset_denom,
        quote_asset_denom,
    }
    .query(&querier)
    .expect(&format!(
        "spot price for {} in pair {}",
        swap_denom, pair.pool_id
    ))
    .spot_price
    .parse::<Decimal>()
}

pub fn query_price(
    querier: &QuerierWrapper,
    env: &Env,
    pair: &Pair,
    swap_amount: &Coin,
) -> StdResult<Decimal> {
    if ![pair.base_denom.clone(), pair.quote_denom.clone()].contains(&swap_amount.denom) {
        return Err(StdError::generic_err(format!(
            "Provided swap denom {} not in pair {}",
            swap_amount.denom, pair.pool_id
        )));
    }

    let token_out_denom = if swap_amount.denom == pair.base_denom {
        pair.quote_denom.clone()
    } else {
        pair.base_denom.clone()
    };

    let routes = vec![SwapAmountInRoute {
        pool_id: pair.pool_id,
        token_out_denom: token_out_denom.clone(),
    }];

    let token_out_amount = PoolmanagerQuerier::new(&querier)
        .estimate_swap_exact_amount_in(
            env.contract.address.to_string(),
            pair.pool_id,
            swap_amount.to_string(),
            routes.clone(),
        )
        .expect(&format!(
            "amount of {} received for swapping {} via {:#?}",
            token_out_denom,
            swap_amount.to_string(),
            routes,
        ))
        .token_out_amount
        .parse::<Uint128>()?;

    Ok(Decimal::from_ratio(swap_amount.amount, token_out_amount))
}

pub fn calculate_slippage(actual_price: Decimal, belief_price: Decimal) -> Decimal {
    let difference = actual_price
        .checked_sub(belief_price)
        .unwrap_or(Decimal::zero());

    if difference.is_zero() {
        return Decimal::zero();
    }

    difference / belief_price
}

pub fn create_osmosis_swap_message(
    querier: QuerierWrapper,
    env: &Env,
    pair: Pair,
    swap_amount: Coin,
    slippage_tolerance: Option<Decimal>,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let token_out_denom = if swap_amount.denom == pair.base_denom {
        pair.quote_denom.clone()
    } else {
        pair.base_denom.clone()
    };

    let token_out_min_amount = slippage_tolerance
        .map_or(Uint128::one(), |slippage_tolerance| {
            let belief_price = query_belief_price(&querier, &pair, &swap_amount.denom)
                .expect("belief price of the pair");
            swap_amount.amount
                * (Decimal::one() / belief_price)
                * (Decimal::one() - Decimal::from_str(OSMOSIS_SWAP_FEE_RATE).unwrap())
                * (Decimal::one() - slippage_tolerance)
        })
        .to_string();

    let swap = MsgSwapExactAmountIn {
        sender: env.contract.address.to_string(),
        token_in: Some(swap_amount.clone().into()),
        token_out_min_amount,
        routes: vec![SwapAmountInRoute {
            pool_id: pair.pool_id,
            token_out_denom,
        }],
    };

    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: swap.into(),
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}

pub fn create_osmosis_liquidity_provision_message(
    sender: String,
    pool_id: u64,
    amount_in: Coin,
    minimum_receive_amount: Option<Uint128>,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let join_liquidity_pair = MsgJoinSwapExternAmountIn {
        sender,
        pool_id,
        token_in: Some(amount_in.into()),
        share_out_min_amount: minimum_receive_amount.unwrap_or(Uint128::one()).into(),
    };

    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: join_liquidity_pair.into(),
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}

pub fn create_osmosis_lock_token_message(
    querier: QuerierWrapper,
    env: Env,
    pool_id: u64,
    owner: Addr,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let balance = querier.query_balance(
        env.contract.address.clone(),
        format!("gamm/pool/{}", pool_id),
    )?;

    let seconds_in_24_hours = 86400;

    let lock = MsgLockTokens {
        owner: owner.to_string(),
        duration: Some(Duration {
            seconds: seconds_in_24_hours,
            nanos: 0,
        }),
        coins: vec![balance.into()],
    };
    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: lock.into(),
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}
