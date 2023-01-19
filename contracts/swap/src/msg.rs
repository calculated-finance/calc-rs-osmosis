use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::{state::config::Config, types::pair::Pair};

#[cw_serde]
pub struct MigrateMsg {
    pub admin: Addr,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig { admin: Addr, paused: bool },
    AddPath { denoms: [String; 2], pair: Pair },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    GetConfig {},
    #[returns(Vec<Pair>)]
    GetPath { denoms: [String; 2] },
}
