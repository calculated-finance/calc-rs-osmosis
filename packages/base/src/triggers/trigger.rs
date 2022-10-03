use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Trigger<T> {
    pub id: Uint128,
    pub owner: Addr,
    pub variant: TriggerVariant,
    pub vault_id: Uint128,
    pub configuration: T,
}

pub struct TriggerBuilder<T> {
    pub id: Uint128,
    pub owner: Addr,
    pub variant: TriggerVariant,
    pub vault_id: Uint128,
    pub configuration: T,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TriggerVariant {
    Time,
    Price,
}