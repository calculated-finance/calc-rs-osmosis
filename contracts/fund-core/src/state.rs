use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub router: Addr,
    pub swapper: Addr,
    pub base_denom: String,
}

const CONFIG: Item<Config> = Item::new("config_v1");

pub fn get_config(store: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(store)
}

pub fn update_config(store: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(store, config)
}
