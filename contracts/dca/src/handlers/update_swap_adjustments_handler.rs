use crate::{
    error::ContractError, state::swap_adjustments::update_swap_adjustments,
    types::position_type::PositionType,
};
use cosmwasm_std::{Decimal, DepsMut, Env, Response};

pub fn update_swap_adjustments_handler(
    deps: DepsMut,
    env: Env,
    position_type: PositionType,
    adjustments: Vec<(u8, Decimal)>,
) -> Result<Response, ContractError> {
    update_swap_adjustments(deps.storage, position_type, adjustments, env.block.time)?;
    Ok(Response::new())
}
