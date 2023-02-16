use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Cache {
    pub owner: Option<Addr>,
    pub router_address: Option<Addr>,
}

pub const CACHE: Item<Cache> = Item::new("cache_v1");

#[cw_serde]
pub struct MigrationCache {
    pub router_address: Addr,
    pub old_fund_address: Addr,
    pub new_fund_address: Option<Addr>,
}

pub const MIGRATION_CACHE: Item<MigrationCache> = Item::new("migration_cache_v1");
