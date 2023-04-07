use crate::types::{pair::Pair, position_type::PositionType};
use cosmwasm_std::{Coin, Decimal, Env, QuerierWrapper, StdResult, Uint128};
use osmosis_std::types::osmosis::{
    gamm::v2::QuerySpotPriceRequest, poolmanager::v1beta1::PoolmanagerQuerier,
};

use super::route_helpers::{calculate_route, get_token_out_denom};

pub fn query_belief_price(
    querier: &QuerierWrapper,
    pair: &Pair,
    mut swap_denom: String,
) -> StdResult<Decimal> {
    let pool_ids = match pair.position_type(swap_denom.clone()) {
        PositionType::Enter => pair.route.clone(),
        PositionType::Exit => pair.route.clone().into_iter().rev().collect(),
    };

    let mut price = Decimal::one();

    for pool_id in pool_ids.into_iter() {
        let target_denom = get_token_out_denom(querier, swap_denom.clone(), pool_id)?;

        price = QuerySpotPriceRequest {
            pool_id,
            base_asset_denom: target_denom.clone(),
            quote_asset_denom: swap_denom,
        }
        .query(&querier)?
        .spot_price
        .parse::<Decimal>()?
            * price;

        swap_denom = target_denom;
    }

    Ok(price)
}

pub fn query_price(
    querier: &QuerierWrapper,
    env: &Env,
    pair: &Pair,
    swap_amount: &Coin,
) -> StdResult<Decimal> {
    let routes = calculate_route(querier, pair, swap_amount.denom.clone())?;

    let token_out_amount = PoolmanagerQuerier::new(&querier)
        .estimate_swap_exact_amount_in(
            env.contract.address.to_string(),
            0,
            swap_amount.to_string(),
            routes.clone(),
        )
        .expect(&format!(
            "amount of {} received for swapping {} via {:#?}",
            routes.last().unwrap().token_out_denom,
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
