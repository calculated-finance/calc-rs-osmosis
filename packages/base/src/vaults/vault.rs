use cosmwasm_std::{Addr, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    Enter,
    Exit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum VaultStatus {
    Active,
    Inactive,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vault<T> {
    pub id: Uint128,
    pub created_at: Timestamp,
    pub owner: Addr,
    pub configuration: T,
    pub status: VaultStatus,
    pub trigger_id: Option<Uint128>,
}
