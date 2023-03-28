use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::LockupDuration;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub allowed_z_callers: Vec<Addr>,
}

pub const CONFIG: Item<Config> = Item::new("config_v1");

#[cw_serde]
pub struct LPCache {
    pub pool_id: u64,
    pub sender_address: Addr,
    pub duration: LockupDuration,
}

pub const LP_CACHE: Item<LPCache> = Item::new("lp_cache_v1");
