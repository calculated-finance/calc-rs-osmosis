use cosmwasm_std::{DepsMut, MessageInfo, Response};

use crate::{
    errors::contract_error::ContractError,
    state::config::{update_config, Config},
    validation::assert_sender_is_admin,
};

pub fn update_config_handler(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;
    deps.api.addr_validate(&config.admin.to_string())?;
    update_config(deps.storage, config)?;
    Ok(Response::new().add_attribute("method", "update_config"))
}
