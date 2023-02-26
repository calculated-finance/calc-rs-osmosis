use cosmwasm_std::{Decimal, StdResult, Storage};
use cw_storage_plus::Map;

const SWAP_ADJUSTMENTS: Map<u8, Decimal> = Map::new("swap_adjustments_v20");

pub fn update_swap_adjustments(
    storage: &mut dyn Storage,
    adjustments: Vec<(u8, Decimal)>,
) -> StdResult<()> {
    for (model, adjustment) in adjustments {
        SWAP_ADJUSTMENTS.save(storage, model, &adjustment)?;
    }
    Ok(())
}

pub fn get_swap_adjustment(storage: &dyn Storage, model: u8) -> StdResult<Decimal> {
    SWAP_ADJUSTMENTS.load(storage, model)
}
