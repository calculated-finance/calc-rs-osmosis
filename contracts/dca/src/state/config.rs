use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub fee_collector: Addr,
    pub fee_percent: Decimal,
    pub staking_router_address: Addr,
    pub page_limit: u16,
}

pub const CONFIG: Item<Config> = Item::new("config_v2");
