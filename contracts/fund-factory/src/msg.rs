use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::state::config::Config;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub fund_router_code_id: u64,
    pub fund_core_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateManagedFund {
        token_name: String,
    },
    UpdateConfig {
        admin: Option<Addr>,
        fund_router_code_id: Option<u64>,
        fund_core_code_id: Option<u64>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(FundRoutersResponse)]
    GetFundRouters { owner: Addr },
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct FundRoutersResponse {
    pub fund_routers: Vec<Addr>,
}
