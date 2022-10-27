use crate::error::ContractError;
use crate::state::pairs::PAIRS;
use crate::validation_helpers::assert_sender_is_admin;
use cosmwasm_std::{Addr, DepsMut};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Env, MessageInfo, Response};

pub fn delete_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    deps.api.addr_validate(&address.to_string())?;

    PAIRS.remove(deps.storage, address.clone());

    Ok(Response::new()
        .add_attribute("method", "delete_pair")
        .add_attribute("address", address.to_string()))
}
