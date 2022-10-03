use cosmwasm_std::{Decimal256, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FINLimitOrderTrigger {
    pub target_price: Decimal256,
    pub order_idx: Uint128,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FINPriceTrigger {
    pub target_price: Decimal256,
    pub order_idx: Uint128,
}
