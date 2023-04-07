use crate::types::{pair::Pair, position_type::PositionType};
use cosmwasm_std::{QuerierWrapper, StdError, StdResult};
use osmosis_std::types::osmosis::{
    gamm::v1beta1::{GammQuerier, Pool},
    poolmanager::v1beta1::SwapAmountInRoute,
};
use prost::DecodeError;

pub fn get_token_out_denom(
    querier: &QuerierWrapper,
    token_in_denom: String,
    pool_id: u64,
) -> StdResult<String> {
    let pool = get_pool(querier, pool_id)?;

    pool.pool_assets
        .iter()
        .find(|asset| asset.token.clone().unwrap().denom != token_in_denom)
        .map(|asset| asset.token.clone().unwrap().denom.clone())
        .ok_or(StdError::generic_err("no token out denom found"))
}

fn get_pool(querier: &QuerierWrapper, pool_id: u64) -> StdResult<Pool> {
    GammQuerier::new(querier)
        .pool(pool_id)?
        .pool
        .expect(&format!("pool id {}", pool_id))
        .try_into()
        .map_err(|e: DecodeError| StdError::ParseErr {
            target_type: Pool::TYPE_URL.to_string(),
            msg: e.to_string(),
        })
}

pub fn calculate_route(
    querier: &QuerierWrapper,
    pair: &Pair,
    mut swap_denom: String,
) -> StdResult<Vec<SwapAmountInRoute>> {
    let pool_ids = match pair.position_type(swap_denom.clone()) {
        PositionType::Enter => pair.route.clone(),
        PositionType::Exit => pair.route.clone().into_iter().rev().collect(),
    };

    let mut route: Vec<SwapAmountInRoute> = vec![];

    for pool_id in pool_ids.into_iter() {
        let target_denom = get_token_out_denom(querier, swap_denom, pool_id)?;

        route.push(SwapAmountInRoute {
            pool_id,
            token_out_denom: target_denom.clone(),
        });

        swap_denom = target_denom;
    }

    Ok(route)
}
