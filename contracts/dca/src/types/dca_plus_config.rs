use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub struct DCAPlusConfig {
    pub direction: DCAPlusDirection,
    pub escrow_level: Decimal,
    pub model_id: u8,
    pub standard_dca_received_amount: Uint128,
    pub escrowed_balance: Uint128,
}

#[cw_serde]
#[derive(Copy)]
pub enum DCAPlusDirection {
    In = 0,
    Out = 1,
}
