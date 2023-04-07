use crate::helpers::validation_helpers::assert_sender_is_admin;
use crate::state::pairs::PAIRS;
use crate::{error::ContractError, types::pair::Pair};
use cosmwasm_std::{Addr, DepsMut};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Env, MessageInfo, Response};

pub fn create_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pool_id: u64,
    address: Addr,
    base_denom: String,
    quote_denom: String,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    let pair = Pair {
        pool_id: pool_id.clone(),
        address: address.clone(),
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
    };

    let existing_pair = PAIRS.may_load(deps.storage, address.clone())?;

    match existing_pair {
        Some(_) => Err(ContractError::CustomError {
            val: format!("pair already exists for address {}", address),
        }),
        None => {
            PAIRS.save(deps.storage, address.clone(), &pair)?;
            Ok(Response::new()
                .add_attribute("method", "create_pair")
                .add_attribute("pool_id", pool_id.to_string())
                .add_attribute("address", address.to_string())
                .add_attribute("base_denom", base_denom)
                .add_attribute("quote_denom", quote_denom))
        }
    }
}
