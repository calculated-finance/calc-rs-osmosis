use cosmwasm_std::{Uint128, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Event<T> {
    pub vault_id: Uint128,
    pub sequence_number: u16,
    pub block_height: Uint64,
    pub event_info: Option<T>,
}

pub struct EventBuilder<T> {
    pub vault_id: Uint128,
    pub sequence_number: u16,
    pub block_height: Uint64,
    pub event_info: Option<T>,
}
