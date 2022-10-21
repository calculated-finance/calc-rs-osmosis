use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal256, Uint128, Uint256};

#[cw_serde]
pub struct FINPoolResponseWithoutDenom {
    pub quote_price: Decimal256,
    pub total_offer_amount: Uint256,
}

#[cw_serde]
pub struct FINBookResponse {
    pub base: Vec<FINPoolResponseWithoutDenom>,
    pub quote: Vec<FINPoolResponseWithoutDenom>,
}

#[cw_serde]
pub struct FINOrderResponseWithoutDenom {
    pub offer_amount: Uint128,
    pub filled_amount: Uint128,
    pub original_offer_amount: Uint128,
}
