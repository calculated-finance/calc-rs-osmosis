use cosmwasm_std::{Decimal, StdResult, Storage};
use cw_storage_plus::Map;
use fin_helpers::position_type::PositionType;

const BUY_ADJUSTMENTS: Map<u8, Decimal> = Map::new("buy_adjustments_v20");
const SELL_ADJUSTMENTS: Map<u8, Decimal> = Map::new("sell_adjustments_v20");

pub fn adjustments_store(position_type: PositionType) -> &'static Map<'static, u8, Decimal> {
    match position_type {
        PositionType::Enter => &BUY_ADJUSTMENTS,
        PositionType::Exit => &SELL_ADJUSTMENTS,
    }
}

pub fn update_swap_adjustments(
    storage: &mut dyn Storage,
    position_type: PositionType,
    adjustments: Vec<(u8, Decimal)>,
) -> StdResult<()> {
    for (model, adjustment) in adjustments {
        adjustments_store(position_type.clone()).save(storage, model, &adjustment)?;
    }
    Ok(())
}

pub fn get_swap_adjustment(
    storage: &dyn Storage,
    position_type: PositionType,
    model: u8,
) -> StdResult<Decimal> {
    adjustments_store(position_type).load(storage, model)
}
