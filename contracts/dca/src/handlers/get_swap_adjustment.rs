use crate::{
    helpers::vault::get_position_type,
    state::swap_adjustments::get_swap_adjustment,
    types::{swap_adjustment_strategy::SwapAdjustmentStrategy, vault::Vault},
};
use cosmwasm_std::{Decimal, Deps, Env, StdResult};

pub fn get_swap_adjustment_handler(deps: &Deps, env: &Env, vault: &Vault) -> StdResult<Decimal> {
    match vault.swap_adjustment_strategy {
        Some(SwapAdjustmentStrategy::DcaPlus { model_id }) => {
            let position_type = get_position_type(deps, vault)?;

            get_swap_adjustment(deps.storage, position_type, model_id, env.block.time)
        }
        None => Ok(Decimal::one()),
    }
}
