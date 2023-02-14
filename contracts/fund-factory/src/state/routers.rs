use cosmwasm_std::{Addr, Order, StdResult, Storage};
use cw_storage_plus::{Item, Map};

use super::state_helpers::fetch_and_increment_counter;

const ROUTER_COUNTER: Item<u64> = Item::new("router_counter_v1");

pub const ROUTERS: Map<(Addr, u64), Addr> = Map::new("routers_v1");

pub fn save_router(
    storage: &mut dyn Storage,
    owner: Addr,
    router_address: Addr,
) -> StdResult<Addr> {
    let id = fetch_and_increment_counter(storage, ROUTER_COUNTER)?;
    ROUTERS.save(storage, (owner.clone(), id), &router_address)?;
    Ok(router_address)
}

pub fn get_routers_by_address(storage: &dyn Storage, address: Addr) -> StdResult<Vec<Addr>> {
    Ok(ROUTERS
        .prefix(address)
        .range(storage, None, None, Order::Ascending)
        .flat_map(|router| router.map(|(_id, router)| router))
        .collect::<Vec<Addr>>())
}
