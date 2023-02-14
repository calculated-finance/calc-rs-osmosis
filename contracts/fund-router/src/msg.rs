use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub token_name: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AssignFund { fund_address: Addr },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(FundResponse)]
    GetFund {},
}

#[cw_serde]
pub struct FundResponse {
    pub address: Addr,
}
