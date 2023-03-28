use crate::error::ContractError;

#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, Reply, Response};

pub fn after_z_liquidity_provision(
    _deps: DepsMut,
    _env: Env,
    _reply: Reply,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "after_z_liquidity_provision"))
}
