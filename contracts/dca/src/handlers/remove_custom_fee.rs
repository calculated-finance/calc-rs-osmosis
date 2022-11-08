use crate::validation_helpers::assert_sender_is_admin;
use crate::{error::ContractError, state::config::remove_custom_fee};
use cosmwasm_std::DepsMut;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{MessageInfo, Response};

pub fn remove_custom_fee_handler(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    remove_custom_fee(deps.storage, denom.clone());

    Ok(Response::new()
        .add_attribute("method", "remove_custom_fee")
        .add_attribute("denom", denom))
}
