use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Cache {
    pub owner: Addr,
    pub router_address: Option<Addr>,
}

pub const CACHE: Item<Cache> = Item::new("cache_v1");
