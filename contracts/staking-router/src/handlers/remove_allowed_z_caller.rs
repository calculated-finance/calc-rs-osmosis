use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, StdResult};

use crate::{
    state::{Config, CONFIG},
    validation_helpers::assert_sender_is_admin,
    ContractError,
};

pub fn remove_allowed_z_caller(
    deps: DepsMut,
    info: MessageInfo,
    allowed_z_caller: Addr,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;
    CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        config
            .allowed_z_callers
            .retain(|z| z.as_ref() != allowed_z_caller);

        Ok(config)
    })?;

    Ok(Response::new().add_attribute("method", "remove_allowed_z_caller"))
}
