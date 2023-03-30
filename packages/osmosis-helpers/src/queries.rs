use crate::{msg::FinConfigResponse, position_type::PositionType};
use base::{pool::Pool, price_type::PriceType};
use cosmwasm_std::{Coin, Decimal, QuerierWrapper, StdError, StdResult};
use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceRequest;

fn _query_quote_price(
    _querier: QuerierWrapper,
    _pool: &Pool,
    _swap_denom: &str,
) -> StdResult<Decimal> {
    unimplemented!()
}

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
        PositionType::Enter => (&pool.base_denom, &pool.quote_denom),
        PositionType::Exit => (&pool.quote_denom, &pool.base_denom),
    };

    QuerySpotPriceRequest {
        pool_id: pool.pool_id,
        base_asset_denom: base_asset_denom.clone(),
        quote_asset_denom: quote_asset_denom.clone(),
    }
    .query(&querier)
    .expect(format!("pool for {} not found", pool.pool_id).as_str())
    .spot_price
    .parse::<Decimal>()
}

pub fn query_price(
    querier: QuerierWrapper,
    pool: Pool,
    swap_amount: &Coin,
    _price_type: PriceType,
) -> StdResult<Decimal> {
    if ![pool.base_denom.clone(), pool.quote_denom.clone()].contains(&swap_amount.denom) {
        return Err(StdError::generic_err(format!(
            "Provided swap denom {} not in pool {}",
            swap_amount.denom, pool.pool_id
        )));
    }

    let position_type = match swap_amount.denom == pool.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let base_asset_denom;
    let quote_asset_denom;

    match position_type {
        PositionType::Enter => {
            base_asset_denom = pool.base_denom;
            quote_asset_denom = pool.quote_denom;
        }
        PositionType::Exit => {
            base_asset_denom = pool.quote_denom;
            quote_asset_denom = pool.base_denom;
        }
    }

    QuerySpotPriceRequest {
        pool_id: pool.pool_id,
        base_asset_denom,
        quote_asset_denom,
    }
    .query(&querier)
    .expect(format!("pool for {} not found", pool.pool_id).as_str())
    .spot_price
    .parse::<Decimal>()
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

pub fn query_pool_config(_querier: QuerierWrapper, _pool_id: u64) -> StdResult<FinConfigResponse> {
    unimplemented!()
}
