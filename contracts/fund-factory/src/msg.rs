use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::state::config::Config;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub router_code_id: u64,
    pub fund_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateRouter {
        token_name: String,
    },
    UpdateConfig {
        admin: Option<Addr>,
        router_code_id: Option<u64>,
        fund_code_id: Option<u64>,
    },
    MigrateToLatestCodeId {
        router: Addr,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(RoutersResponse)]
    GetRouters { owner: Addr },
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct RoutersResponse {
    pub routers: Vec<Addr>,
}
