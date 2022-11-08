use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub fee_collector: Addr,
    pub fee_percent: Decimal,
    pub staking_router_address: Addr,
    pub page_limit: u16,
    pub paused: bool,
}

const CONFIG: Item<Config> = Item::new("config_v4");

pub fn get_config(store: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(store)
}

pub fn update_config(store: &mut dyn Storage, config: Config) -> StdResult<Config> {
    if config.fee_percent > Decimal::percent(100) {
        return Err(StdError::generic_err(
            "fee_percent must be less than 100%, and expressed as a ratio out of 1 (i.e. use 0.015 to represent a fee of 1.5%)",
        ));
    }

    CONFIG.save(store, &config)?;
    Ok(config)
}

pub fn clear_config(store: &mut dyn Storage) {
    CONFIG.remove(store);
}

const CUSTOM_FEES: Map<String, Decimal> = Map::new("fees_v1");

pub fn create_custom_fee(
    storage: &mut dyn Storage,
    denom: String,
    fee_percent: Decimal,
) -> StdResult<()> {
    if fee_percent > Decimal::percent(100) {
        return Err(StdError::generic_err(
            "fee_percent must be less than 100%, and expressed as a ratio out of 1 (i.e. use 0.015 to represent a fee of 1.5%)",
        ));
    }

    CUSTOM_FEES.save(storage, denom, &fee_percent)
}

pub fn remove_custom_fee(storage: &mut dyn Storage, denom: String) {
    CUSTOM_FEES.remove(storage, denom);
}

pub fn get_custom_fee(storage: &dyn Storage, denom: String) -> Option<Decimal> {
    CUSTOM_FEES.may_load(storage, denom).unwrap()
}

pub fn get_custom_fees(storage: &dyn Storage) -> StdResult<Vec<(String, Decimal)>> {
    CUSTOM_FEES
        .range(storage, None, None, Order::Ascending)
        .collect()
}
