use cosmwasm_std::{Decimal, Env, StdResult, Storage};

use crate::{
    state::{pairs::find_pair, swap_adjustments::get_swap_adjustment},
    types::swap_adjustment_strategy::SwapAdjustmentStrategy,
};

pub fn get_swap_adjustment_handler(
    storage: &dyn Storage,
    env: &Env,
    swap_adjustment_strategy: Option<SwapAdjustmentStrategy>,
) -> StdResult<Decimal> {
    match swap_adjustment_strategy {
        Some(SwapAdjustmentStrategy::DcaPlus {
            model_id,
            standard_dca_swapped_amount,
            standard_dca_received_amount,
            ..
        }) => {
            let pair = find_pair(
                storage,
                &[
                    standard_dca_swapped_amount.denom.clone(),
                    standard_dca_received_amount.denom,
                ],
            )?;

            let position_type = pair.position_type(standard_dca_swapped_amount.denom);

            get_swap_adjustment(storage, position_type, model_id, env.block.time)
        }
        None => Ok(Decimal::one()),
    }
}
