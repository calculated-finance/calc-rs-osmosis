use super::post_execution_action::PostExecutionAction;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_binary, to_binary, Addr, Binary, Decimal};

#[cw_serde]
pub struct Destination {
    pub address: Addr,
    pub allocation: Decimal,
    pub action: PostExecutionAction,
}

impl From<Destination> for Binary {
    fn from(destination: Destination) -> Self {
        to_binary(&destination).expect(&format!("serialised destination {:#?}", destination))
    }
}

impl From<Binary> for Destination {
    fn from(binary: Binary) -> Self {
        from_binary(&binary).expect(&format!("deserialised destination {:#?}", binary))
    }
}
