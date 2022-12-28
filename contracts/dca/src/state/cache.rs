use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal256, Timestamp, Uint128};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Cache {
    pub vault_id: Uint128,
    pub owner: Addr,
}

#[cw_serde]
pub struct LimitOrderCache {
    pub order_idx: Uint128,
    pub offer_amount: Uint128,
    pub original_offer_amount: Uint128,
    pub filled: Uint128,
    pub quote_price: Decimal256,
    pub created_at: Timestamp,
    pub swap_denom_balance: Coin,
    pub receive_denom_balance: Coin,
}

pub const CACHE: Item<Cache> = Item::new("cache_v20");

pub const LIMIT_ORDER_CACHE: Item<LimitOrderCache> = Item::new("limit_order_cache_v20");

#[cw_serde]
pub struct SwapCache {
    pub swap_denom_balance: Coin,
    pub receive_denom_balance: Coin,
}

pub const SWAP_CACHE: Item<SwapCache> = Item::new("swap_cache_v20");
