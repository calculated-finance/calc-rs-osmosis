use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::triggers::trigger::TriggerVariant;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vault<T, S> {
    pub id: Uint128,
    pub created_at: Timestamp,
    pub owner: Addr,
    pub balances: Vec<Coin>,
    pub configuration: T,
    pub status: S,
    pub trigger_id: Uint128,
    pub trigger_variant: TriggerVariant,
}

pub struct VaultBuilder<T, S> {
    pub id: Uint128,
    pub owner: Addr,
    pub created_at: Timestamp,
    pub balances: Vec<Coin>,
    pub configuration: T,
    pub status: S,
    pub trigger_id: Uint128,
    pub trigger_variant: TriggerVariant, // used to know where to do trigger lookup
}
