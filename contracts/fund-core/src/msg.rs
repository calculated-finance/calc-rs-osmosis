use crate::types::failure_behaviour::FailureBehaviour;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Decimal256};

#[cw_serde]
pub struct InstantiateMsg {
    pub router: Addr,
    pub swapper: Addr,
    pub base_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Rebalance {
        allocations: Vec<(String, Decimal)>,
        slippage_tolerance: Option<Decimal256>,
        failure_behaviour: Option<FailureBehaviour>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
