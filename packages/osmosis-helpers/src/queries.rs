use base::pool::Pool;
use cosmwasm_std::{Coin, Decimal, Env, QuerierWrapper, StdError, StdResult, Uint128};
use osmosis_std::types::osmosis::poolmanager::v1beta1::{PoolmanagerQuerier, SwapAmountInRoute};

pub fn query_belief_price(
    querier: QuerierWrapper,
    env: &Env,
    pool: &Pool,
    swap_denom: &str,
) -> StdResult<Decimal> {
    query_price(
        querier,
        env,
        pool,
        &Coin::new(Uint128::new(100).into(), swap_denom),
    )
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
