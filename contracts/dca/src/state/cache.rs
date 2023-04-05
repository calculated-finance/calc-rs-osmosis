use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Cache {
    pub vault_id: Uint128,
    pub owner: Addr,
}

pub const CACHE: Item<Cache> = Item::new("cache_v3");

#[cw_serde]
pub struct SwapCache {
    pub swap_denom_balance: Coin,
    pub receive_denom_balance: Coin,
}

pub const SWAP_CACHE: Item<SwapCache> = Item::new("swap_cache_v3");
