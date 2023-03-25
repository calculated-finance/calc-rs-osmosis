use crate::error::ContractError;
use crate::helpers::validation_helpers::assert_sender_is_admin;
use crate::state::pools::POOLS;
use cosmwasm_std::{DepsMut};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Env, MessageInfo, Response};

pub fn delete_pool(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pool_id: u64,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    POOLS.remove(deps.storage, pool_id.clone());

    Ok(Response::new()
        .add_attribute("method", "delete_pool")
        .add_attribute("pool_id", pool_id.to_string()))
}
