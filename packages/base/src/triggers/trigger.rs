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
#[serde(rename_all = "snake_case")]
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
pub struct Trigger {
    pub vault_id: Uint128,
    pub configuration: TriggerConfiguration,
}
