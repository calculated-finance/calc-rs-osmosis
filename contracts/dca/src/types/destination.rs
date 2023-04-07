use super::post_execution_action::PostExecutionAction;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

#[cw_serde]
pub struct Destination {
    pub address: Addr,
    pub allocation: Decimal,
    pub action: PostExecutionAction,
}
