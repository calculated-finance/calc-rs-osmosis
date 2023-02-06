use crate::types::callback::Callback;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use cw_storage_plus::Item;

#[cw_serde]
pub struct SwapCache {
    pub callback: Callback,
    pub receive_denom_balance: Coin,
}

pub const SWAP_CACHE: Item<SwapCache> = Item::new("swap_cache_v20");
