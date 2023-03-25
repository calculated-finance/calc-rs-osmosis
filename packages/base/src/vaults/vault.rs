use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};

#[cw_serde]
pub enum VaultStatus {
    Scheduled,
    Active,
    Inactive,
    Cancelled,
}

#[cw_serde]
pub enum PostExecutionActionDeprecated {
    Send,
    ZDelegate,
}

#[cw_serde]
pub enum PostExecutionAction {
    Send,
    ZDelegate,
}

#[cw_serde]
pub struct DestinationDeprecated {
    pub address: Addr,
    pub allocation: Decimal,
    pub action: PostExecutionActionDeprecated,
}

#[cw_serde]
pub struct Destination {
    pub address: Addr,
    pub allocation: Decimal,
    pub action: PostExecutionAction,
}

impl From<Destination> for DestinationDeprecated {
    fn from(destination: Destination) -> Self {
        DestinationDeprecated {
            address: destination.address,
            allocation: destination.allocation,
            action: match destination.action {
                PostExecutionAction::Send => PostExecutionActionDeprecated::Send,
                PostExecutionAction::ZDelegate => PostExecutionActionDeprecated::ZDelegate,
            },
        }
    }
}

impl From<DestinationDeprecated> for Destination {
    fn from(destination: DestinationDeprecated) -> Self {
        Destination {
            address: destination.address,
            allocation: destination.allocation,
            action: match destination.action {
                PostExecutionActionDeprecated::Send => PostExecutionAction::Send,
                PostExecutionActionDeprecated::ZDelegate => PostExecutionAction::ZDelegate,
            },
        }
    }
}
