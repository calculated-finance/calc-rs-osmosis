use cosmwasm_std::{Decimal, Decimal256, Timestamp, Uint128};
use kujira::precision::Precision;
use serde::{Deserialize, Serialize};

// use serde instead of cw_serde so allow for deserialisation of unknown fields
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FinPoolResponseWithoutDenom {
    pub quote_price: Decimal,
    pub total_offer_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FinBookResponse {
    pub base: Vec<FinPoolResponseWithoutDenom>,
    pub quote: Vec<FinPoolResponseWithoutDenom>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FinOrderResponseWithoutDenom {
    pub offer_amount: Uint128,
    pub filled_amount: Uint128,
    pub original_offer_amount: Uint128,
    pub quote_price: Decimal256,
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FinConfigResponse {
    pub decimal_delta: Option<i8>,
    pub price_precision: Precision,
}
