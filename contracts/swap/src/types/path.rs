use std::collections::VecDeque;

use super::pair::Pair;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal256;

#[cw_serde]
pub struct Path {
    pub price: Decimal256,
    pub pairs: VecDeque<Pair>,
}
