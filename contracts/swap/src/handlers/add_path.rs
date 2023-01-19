use cosmwasm_std::{DepsMut, Response};

use crate::{errors::contract_error::ContractError, state::paths::add_path, types::pair::Pair};

pub fn add_path_handler(
    deps: DepsMut,
    denoms: [String; 2],
    pair: Pair,
) -> Result<Response, ContractError> {
    add_path(deps.storage, denoms.clone(), pair.clone())?;
    Ok(Response::new()
        .add_attribute("method", "add_path")
        .add_attribute("denoms", format!("[{}, {}]", denoms[0], denoms[1]))
        .add_attribute("pair", format!("{:?}", pair)))
}
