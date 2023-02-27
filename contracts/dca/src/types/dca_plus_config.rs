use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub struct DCAPlusConfig {
    pub escrow_level: Decimal,
    pub model_id: u8,
    pub amount_withdrawn: Uint128,
}
