use crate::{msg::PoolsResponse, state::pools::POOLS};
use base::pool::Pool;
use cosmwasm_std::{Deps, Order, StdResult};

pub fn get_pools(deps: Deps) -> StdResult<PoolsResponse> {
    let all_pools: StdResult<Vec<_>> = POOLS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();

    let pools: Vec<Pool> = all_pools.unwrap().iter().map(|p| p.1.clone()).collect();

    Ok(PoolsResponse { pools })
}
