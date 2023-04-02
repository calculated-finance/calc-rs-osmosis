use base::pool::Pool;
use cosmwasm_std::{Coin, Decimal, Env, QuerierWrapper, StdError, StdResult, Uint128};
use osmosis_std::types::osmosis::{
    gamm::v2::QuerySpotPriceRequest,
    poolmanager::v1beta1::{PoolmanagerQuerier, SwapAmountInRoute},
};

use crate::position_type::PositionType;

pub fn query_belief_price(
    querier: QuerierWrapper,
    pool: &Pool,
    swap_denom: &str,
) -> StdResult<Decimal> {
    if ![pool.base_denom.clone(), pool.quote_denom.clone()].contains(&swap_denom.to_string()) {
        return Err(StdError::generic_err(format!(
            "Provided swap denom {} not in pool {}",
            swap_denom, pool.pool_id
        )));
    }
    let position_type = match swap_denom == pool.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let (base_asset_denom, quote_asset_denom) = match position_type {
        PositionType::Enter => (pool.base_denom.clone(), pool.quote_denom.clone()),
        PositionType::Exit => (pool.quote_denom.clone(), pool.base_denom.clone()),
    };

    QuerySpotPriceRequest {
        pool_id: pool.pool_id,
        base_asset_denom,
        quote_asset_denom,
    }
    .query(&querier)
    .expect(&format!(
        "spot price for {} in pool {}",
        swap_denom, pool.pool_id
    ))
    .spot_price
    .parse::<Decimal>()
}

pub fn query_price(
    querier: QuerierWrapper,
    env: &Env,
    pool: &Pool,
    swap_amount: &Coin,
) -> StdResult<Decimal> {
    if ![pool.base_denom.clone(), pool.quote_denom.clone()].contains(&swap_amount.denom) {
        return Err(StdError::generic_err(format!(
            "Provided swap denom {} not in pool {}",
            swap_amount.denom, pool.pool_id
        )));
    }

    let token_out_denom = if swap_amount.denom == pool.base_denom {
        pool.quote_denom.clone()
    } else {
        pool.base_denom.clone()
    };

    let token_out_amount = PoolmanagerQuerier::new(&querier)
        .estimate_swap_exact_amount_in(
            env.contract.address.to_string(),
            pool.pool_id,
            swap_amount.to_string(),
            vec![SwapAmountInRoute {
                pool_id: pool.pool_id,
                token_out_denom: token_out_denom.clone(),
            }],
        )
        .expect(&format!(
            "token out amount of {} for swapping {} on pool {} from sender {}",
            token_out_denom,
            swap_amount.to_string(),
            pool.pool_id,
            env.contract.address.to_string()
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
