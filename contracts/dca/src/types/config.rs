use super::fee_collector::FeeCollector;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub executors: Vec<Addr>,
    pub fee_collectors: Vec<FeeCollector>,
    pub default_swap_fee_percent: Decimal,
    pub weighted_scale_swap_fee_percent: Decimal,
    pub automation_fee_percent: Decimal,
    pub default_page_limit: u16,
    pub paused: bool,
    pub risk_weighted_average_escrow_level: Decimal,
    pub twap_period: u64,
    pub default_slippage_tolerance: Decimal,
}
