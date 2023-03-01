use crate::{
    error::ContractError, state::swap_adjustments::update_swap_adjustments,
    types::dca_plus_config::DCAPlusDirection,
};
use cosmwasm_std::{Decimal, DepsMut, Response};

pub fn update_swap_adjustments_handler(
    deps: DepsMut,
    direction: DCAPlusDirection,
    adjustments: Vec<(u8, Decimal)>,
) -> Result<Response, ContractError> {
    update_swap_adjustments(deps.storage, direction, adjustments)?;
    Ok(Response::new())
}
