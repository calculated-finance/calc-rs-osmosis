use cosmwasm_std::{Addr, Order, StdResult, Storage};
use cw_storage_plus::{Item, Map};

use super::state_helpers::fetch_and_increment_counter;

const FUND_ROUTER_COUNTER: Item<u64> = Item::new("fund_router_counter_v1");

pub const FUND_ROUTERS: Map<(Addr, u64), Addr> = Map::new("fund_routers_v1");

pub fn save_fund_router(
    storage: &mut dyn Storage,
    owner: Addr,
    router_address: Addr,
) -> StdResult<Addr> {
    let id = fetch_and_increment_counter(storage, FUND_ROUTER_COUNTER)?;
    FUND_ROUTERS.save(storage, (owner.clone(), id), &router_address)?;
    Ok(router_address)
}

pub fn get_fund_routers_by_address(storage: &dyn Storage, address: Addr) -> StdResult<Vec<Addr>> {
    Ok(FUND_ROUTERS
        .prefix(address)
        .range(storage, None, None, Order::Ascending)
        .flat_map(|fund_router| fund_router.map(|(_id, fund_router)| fund_router))
        .collect::<Vec<Addr>>())
}
