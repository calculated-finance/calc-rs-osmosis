use crate::{error::ContractError, msg::MigrateMsg};
use cosmwasm_std::{DepsMut, Response};

pub fn migrate_handler(_deps: DepsMut, msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new()
        .add_attribute("method", "migrate")
        .add_attribute("msg", format!("{:#?}", msg)))
}
