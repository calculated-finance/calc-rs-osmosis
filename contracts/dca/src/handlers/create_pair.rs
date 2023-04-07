use crate::helpers::validation_helpers::assert_sender_is_admin;
use crate::state::pairs::PAIRS;
use crate::{error::ContractError, types::pair::Pair};
use cosmwasm_std::{Addr, DepsMut};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{MessageInfo, Response};

pub fn create_pair(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
    base_denom: String,
    quote_denom: String,
    route: Vec<u64>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    if route.is_empty() {
        return Err(ContractError::CustomError {
            val: "Swap route must not be empty".to_string(),
        });
    }

    let pair = Pair {
        address: address.clone(),
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        route: route.clone(),
    };

    PAIRS.save(deps.storage, address.clone(), &pair)?;

    Ok(Response::new()
        .add_attribute("method", "create_pair")
        .add_attribute("address", address.to_string())
        .add_attribute("base_denom", base_denom)
        .add_attribute("quote_denom", quote_denom)
        .add_attribute("route", format!("{:#?}", route)))
}
