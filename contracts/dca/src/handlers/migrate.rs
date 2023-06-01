use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    msg::MigrateMsg,
};
use cosmwasm_std::{DepsMut, Response, StdError};
use cw2::{get_contract_version, set_contract_version};

pub fn migrate_handler(deps: DepsMut, msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    if contract_version.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }

    #[allow(clippy::cmp_owned)]
    if contract_version.version > CONTRACT_VERSION.to_string() {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("migrate", "true")
        .add_attribute("msg", format!("{:#?}", msg)))
}
