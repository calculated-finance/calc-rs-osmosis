use cosmwasm_std::{Addr, StdResult, Storage};
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
    FUND_ROUTERS.save(storage, (owner.clone(), id), &owner)?;
    Ok(router_address)
}
