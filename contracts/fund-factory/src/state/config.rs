use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub router_code_id: u64,
}

const CONFIG: Item<Config> = Item::new("config_v1");

pub fn update_config(store: &mut dyn Storage, config: Config) -> StdResult<Config> {
    CONFIG.save(store, &config)?;
    Ok(config)
}

pub fn get_config(store: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(store)
}
