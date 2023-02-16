use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::state::config::Config;
use fund_core::msg::ConfigResponse as FundConfigResponse;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
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
    #[returns(FundConfigResponse)]
    GetFundConfig {},
    #[returns(ConfigResponse)]
    GetConfig {},
}

#[cw_serde]
pub struct FundResponse {
    pub address: Option<Addr>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}
