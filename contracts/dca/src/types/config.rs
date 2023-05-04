use super::fee_collector::FeeCollector;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub executors: Vec<Addr>,
    pub fee_collectors: Vec<FeeCollector>,
    pub swap_fee_percent: Decimal,
    pub delegation_fee_percent: Decimal,
    pub page_limit: u16,
    pub paused: bool,
    pub risk_weighted_average_escrow_level: Decimal,
}
