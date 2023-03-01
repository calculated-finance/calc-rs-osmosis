use cosmwasm_std::{Decimal, StdResult, Storage};
use cw_storage_plus::Map;

use crate::types::dca_plus_config::DCAPlusDirection;

const BUY_ADJUSTMENTS: Map<u8, Decimal> = Map::new("buy_adjustments_v20");
const SELL_ADJUSTMENTS: Map<u8, Decimal> = Map::new("sell_adjustments_v20");

pub fn adjustments_store(direction: DCAPlusDirection) -> &'static Map<'static, u8, Decimal> {
    match direction {
        DCAPlusDirection::In => &BUY_ADJUSTMENTS,
        DCAPlusDirection::Out => &SELL_ADJUSTMENTS,
    }
}

pub fn update_swap_adjustments(
    storage: &mut dyn Storage,
    direction: DCAPlusDirection,
    adjustments: Vec<(u8, Decimal)>,
) -> StdResult<()> {
    for (model, adjustment) in adjustments {
        adjustments_store(direction.clone()).save(storage, model, &adjustment)?;
    }
    Ok(())
}

pub fn get_swap_adjustment(
    storage: &dyn Storage,
    direction: DCAPlusDirection,
    model: u8,
) -> StdResult<Decimal> {
    adjustments_store(direction).load(storage, model)
}
