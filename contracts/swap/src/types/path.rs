use std::collections::VecDeque;

use super::pair::Pair;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

#[cw_serde]
pub struct Path {
    pub price: Decimal,
    pub pairs: VecDeque<Pair>,
}
