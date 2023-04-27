use crate::{
    state::swap_adjustments::get_swap_adjustment,
    types::swap_adjustment_strategy::SwapAdjustmentStrategy,
};
use cosmwasm_std::{Decimal, Deps, Env, StdResult};

pub fn get_swap_adjustment_handler(
    deps: &Deps,
    env: &Env,
    strategy: SwapAdjustmentStrategy,
) -> StdResult<Decimal> {
    Ok(get_swap_adjustment(deps.storage, strategy, env.block.time))
}
