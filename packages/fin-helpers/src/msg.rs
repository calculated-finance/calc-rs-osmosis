use cosmwasm_std::{Decimal256, Uint128, Uint256};
use serde::{Deserialize, Serialize};
// use serde instead of cw_serde so allow for deserialisation of unknown fields
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FINPoolResponseWithoutDenom {
    pub quote_price: Decimal256,
    pub total_offer_amount: Uint256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FINBookResponse {
    pub base: Vec<FINPoolResponseWithoutDenom>,
    pub quote: Vec<FINPoolResponseWithoutDenom>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FINOrderResponseWithoutDenom {
    pub offer_amount: Uint128,
    pub filled_amount: Uint128,
    pub original_offer_amount: Uint128,
}
