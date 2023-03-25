use crate::{
    error::ContractError, helpers::validation_helpers::assert_sender_is_admin,
    state::config::create_custom_fee,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::Response;
use cosmwasm_std::{Decimal, DepsMut, MessageInfo};

pub fn create_custom_swap_fee(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    swap_fee_percent: Decimal,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    create_custom_fee(deps.storage, denom.clone(), swap_fee_percent)?;

    Ok(Response::new()
        .add_attribute("method", "create_custom_swap_fee")
        .add_attribute("denom", denom)
        .add_attribute("swap_fee_percent", swap_fee_percent.to_string()))
}
