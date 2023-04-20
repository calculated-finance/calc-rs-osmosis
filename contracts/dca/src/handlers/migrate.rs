use crate::error::ContractError;
use cosmwasm_std::Response;

pub fn migrate_handler() -> Result<Response, ContractError> {
    Ok(Response::new())
}
