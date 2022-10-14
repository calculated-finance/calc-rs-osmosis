use crate::validation_helpers::assert_sender_is_admin;
use crate::{error::ContractError, state::PAIRS};
use cosmwasm_std::{Addr, DepsMut};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Env, MessageInfo, Response};

pub fn delete_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.as_ref(), info.sender)?;

    let validated_pair_address: Addr = deps.api.addr_validate(&address)?;

    PAIRS.remove(deps.storage, validated_pair_address.clone());

    Ok(Response::new()
        .add_attribute("method", "delete_pair")
        .add_attribute("address", validated_pair_address.to_string()))
}
