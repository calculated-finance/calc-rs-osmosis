use crate::{
    error::ContractError, state::config::create_custom_fee,
    validation_helpers::assert_sender_is_admin,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::Response;
use cosmwasm_std::{Decimal, DepsMut, MessageInfo};

pub fn create_custom_fee_handler(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    fee_percent: Decimal,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    create_custom_fee(deps.storage, denom.clone(), fee_percent)?;

    Ok(Response::new()
        .add_attribute("method", "create_custom_fee")
        .add_attribute("denom", denom)
        .add_attribute("fee_percent", fee_percent.to_string()))
}
