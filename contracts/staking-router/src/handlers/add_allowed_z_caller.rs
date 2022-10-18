use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, StdResult};

use crate::{
    state::{Config, CONFIG},
    validation_helpers::assert_sender_is_admin,
    ContractError,
};

pub fn add_allowed_z_caller(
    deps: DepsMut,
    info: MessageInfo,
    allowed_z_caller: Addr,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        match config.allowed_z_callers.contains(&allowed_z_caller) {
            true => Ok(config),
            false => {
                config.allowed_z_callers.push(allowed_z_caller);
                Ok(config)
            }
        }
    })?;

    Ok(Response::new().add_attribute("method", "add_allowed_z_caller"))
}
