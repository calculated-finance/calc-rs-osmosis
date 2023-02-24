use crate::{error::ContractError, state::swap_adjustments::update_swap_adjustments};
use cosmwasm_std::{Decimal, DepsMut, Response};

pub fn update_swap_adjustments_handler(
    deps: DepsMut,
    adjustments: Vec<(u8, Decimal)>,
) -> Result<Response, ContractError> {
    update_swap_adjustments(deps.storage, adjustments)?;
    Ok(Response::new())
}
