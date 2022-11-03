use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdError, StdResult, Storage};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub fee_collector: Addr,
    pub fee_percent: Decimal,
    pub staking_router_address: Addr,
    pub page_limit: u16,
}

const CONFIG: Item<Config> = Item::new("config_v4");

pub fn get_config(store: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(store)
}

pub fn update_config(store: &mut dyn Storage, config: Config) -> StdResult<Config> {
    if config.fee_percent > Decimal::percent(100) {
        return Err(StdError::generic_err(
            "fee_percent must be less than 100% (i.e. 0.015)",
        ));
    }

    CONFIG.save(store, &config)?;
    Ok(config)
}

pub fn clear_config(store: &mut dyn Storage) {
    CONFIG.remove(store);
}
