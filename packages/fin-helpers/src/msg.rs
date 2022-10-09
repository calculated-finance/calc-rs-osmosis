use cosmwasm_std::{Decimal256, Uint128, Uint256};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FINPoolResponseWithoutDenom {
    pub quote_price: Decimal256,
    pub total_offer_amount: Uint256,
}

#[derive(Deserialize)]
pub struct FINBookResponse {
    pub base: Vec<FINPoolResponseWithoutDenom>,
    pub quote: Vec<FINPoolResponseWithoutDenom>,
}

#[derive(Deserialize)]
pub struct FINOrderResponseWithoutDenom {
    pub offer_amount: Uint128,
    pub filled_amount: Uint128,
    pub original_offer_amount: Uint128,
}
