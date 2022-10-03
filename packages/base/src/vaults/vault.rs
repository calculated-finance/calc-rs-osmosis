use cosmwasm_std::{Addr, Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::triggers::trigger::TriggerVariant;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vault<T> {
    pub id: Uint128,
    pub owner: Addr,
    pub balances: Vec<Balance>,
    pub configuration: T,
    pub trigger_id: Uint128,
    pub trigger_variant: TriggerVariant,
}

pub struct VaultBuilder<T> {
    pub id: Uint128,
    pub owner: Addr,
    pub balances: Vec<Balance>,
    pub configuration: T,
    pub trigger_id: Uint128,
    pub trigger_variant: TriggerVariant, // used to know where to do trigger lookup
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Balance {
    pub starting: Coin,
    pub current: Coin,
}
