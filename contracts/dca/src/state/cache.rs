use std::collections::VecDeque;

use crate::types::lockable_duration::LockableDuration;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, SubMsg, Uint128};
use cw_storage_plus::{Item, Map};

pub const VAULT_CACHE: Item<Uint128> = Item::new("vault_cache_v8");

#[cw_serde]
pub struct SwapCache {
    pub swap_denom_balance: Coin,
    pub receive_denom_balance: Coin,
}

pub const SWAP_CACHE: Item<SwapCache> = Item::new("swap_cache_v8");

#[cw_serde]
pub struct PostExecutionActionCacheEntry {
    pub msg: SubMsg,
    pub funds: Vec<Coin>,
}

pub const POST_EXECUTION_ACTION_CACHE: Map<u128, VecDeque<PostExecutionActionCacheEntry>> =
    Map::new("post_execution_action_cache_v8");

#[cw_serde]
pub struct ProvideLiquidityCache {
    pub provider_address: Addr,
    pub pool_id: u64,
    pub duration: LockableDuration,
}

pub const PROVIDE_LIQUIDITY_CACHE: Item<ProvideLiquidityCache> =
    Item::new("provide_liquidity_cache_v8");
