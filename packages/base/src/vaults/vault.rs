use cosmwasm_std::{Addr, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    Enter,
    Exit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VaultStatus {
    Active,
    Inactive,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PostExecutionAction {
    Send,
    Delegate,
    ZDelegate,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Destination {
    pub address: Addr,
    pub allocation: Decimal,
    pub post_execution_action: PostExecutionAction,
}
