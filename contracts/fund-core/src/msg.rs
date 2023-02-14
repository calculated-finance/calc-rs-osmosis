use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub router: Addr,
    pub swapper: Addr,
    pub base_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
