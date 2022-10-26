use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

#[cw_serde]
pub enum PositionType {
    Enter,
    Exit,
}

#[cw_serde]
pub enum VaultStatus {
    Scheduled,
    Active,
    Inactive,
    Cancelled,
}

#[cw_serde]
pub enum PostExecutionAction {
    Send,
    ZDelegate,
}

#[cw_serde]
pub struct Destination {
    pub address: Addr,
    pub allocation: Decimal,
    pub action: PostExecutionAction,
}
