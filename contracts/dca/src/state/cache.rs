use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::Item;

use crate::types::post_execution_action::LockableDuration;

#[cw_serde]
pub struct VaultCache {
    pub vault_id: Uint128,
}

pub const VAULT_CACHE: Item<VaultCache> = Item::new("vault_cache_v8");

#[cw_serde]
pub struct SwapCache {
    pub swap_denom_balance: Coin,
    pub receive_denom_balance: Coin,
}

pub const SWAP_CACHE: Item<SwapCache> = Item::new("swap_cache_v8");

#[cw_serde]
pub struct ProvideLiquidityCache {
    pub provider_address: Addr,
    pub pool_id: u64,
    pub duration: LockableDuration,
}

pub const PROVIDE_LIQUIDITY_CACHE: Item<ProvideLiquidityCache> =
    Item::new("provide_liquidity_cache_v8");
