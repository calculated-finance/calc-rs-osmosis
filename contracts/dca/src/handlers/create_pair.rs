use crate::helpers::route_helpers::calculate_route;
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

    for denom in pair.denoms() {
        let route = calculate_route(&deps.querier, &pair, denom.clone())?;

        if route.last().unwrap().token_out_denom != pair.other_denom(denom.clone()) {
            return Err(ContractError::CustomError {
                val: format!(
                    "Swap route is invalid. Last token out denom must be {}",
                    pair.other_denom(denom)
                ),
            });
        }
    }

    PAIRS.save(deps.storage, address.clone(), &pair)?;

    Ok(Response::new()
        .add_attribute("method", "create_pair")
        .add_attribute("address", address.to_string())
        .add_attribute("base_denom", base_denom)
        .add_attribute("quote_denom", quote_denom)
        .add_attribute("route", format!("{:#?}", route)))
}
