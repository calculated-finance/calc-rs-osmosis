use cosmwasm_std::{Decimal256, Timestamp, Uint128};
use enum_as_inner::EnumAsInner;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TimeInterval {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, EnumAsInner)]
pub enum TriggerConfiguration {
    Time {
        target_time: Timestamp,
    },
    FINLimitOrder {
        target_price: Decimal256,
        order_idx: Option<Uint128>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum TriggerStatus {
    Active,
    Executed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Trigger {
    pub id: Uint128,
    pub vault_id: Uint128,
    pub configuration: TriggerConfiguration,
    pub status: TriggerStatus,
}

pub struct TriggerBuilder {
    pub vault_id: Uint128,
    pub configuration: TriggerConfiguration,
    pub status: TriggerStatus,
}

impl TriggerBuilder {
    pub fn build(self, id: Uint128) -> Trigger {
        Trigger {
            id,
            vault_id: self.vault_id,
            configuration: self.configuration,
            status: self.status,
        }
    }
}
