use crate::types::swap_adjustment_strategy::SwapAdjustmentStrategy;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdResult, Storage, Timestamp};
use cw_storage_plus::Map;

#[cw_serde]
struct SwapAdjustment {
    adjustment: Decimal,
    timestamp: u64,
}

const SWAP_ADJUSTMENTS: Map<u64, SwapAdjustment> = Map::new("buy_adjustments_v7");

pub fn update_swap_adjustment(
    storage: &mut dyn Storage,
    strategy: SwapAdjustmentStrategy,
    value: Decimal,
    block_time: Timestamp,
) -> StdResult<()> {
    SWAP_ADJUSTMENTS.save(
        storage,
        strategy.hash(),
        &SwapAdjustment {
            adjustment: value,
            timestamp: block_time.seconds(),
        },
    )
}

pub fn get_swap_adjustment(
    storage: &dyn Storage,
    strategy: SwapAdjustmentStrategy,
    block_time: Timestamp,
) -> StdResult<Decimal> {
    let adjustment = SWAP_ADJUSTMENTS
        .load(storage, strategy.hash())
        .unwrap_or_else(|_| SwapAdjustment {
            adjustment: Decimal::one(),
            timestamp: block_time.seconds(),
        });

    let one_day = 25 * 60 * 60;

    if adjustment.timestamp + one_day > block_time.seconds() {
        Ok(adjustment.adjustment)
    } else {
        Ok(Decimal::one())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{position_type::PositionType, swap_adjustment_strategy::BaseDenom};

    use super::*;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Decimal,
    };

    #[test]
    fn gets_swap_adjustment_if_updated_within_a_day() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let adjustment_value = Decimal::percent(90);

        let strategy = SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id: 30,
            base_denom: BaseDenom::Bitcoin,
            position_type: PositionType::Enter,
        };

        update_swap_adjustment(
            deps.as_mut().storage,
            strategy.clone(),
            adjustment_value,
            env.block.time,
        )
        .unwrap();

        let adjustment =
            get_swap_adjustment(deps.as_ref().storage, strategy, env.block.time).unwrap();

        assert_eq!(adjustment, adjustment_value);
    }

    #[test]
    fn gets_default_swap_adjustment_if_not_updated_within_25_hours() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let adjustment_value = Decimal::percent(90);

        let strategy = SwapAdjustmentStrategy::RiskWeightedAverage {
            model_id: 30,
            base_denom: BaseDenom::Bitcoin,
            position_type: PositionType::Enter,
        };

        update_swap_adjustment(
            deps.as_mut().storage,
            strategy.clone(),
            adjustment_value,
            env.block.time,
        )
        .unwrap();

        let adjustment = get_swap_adjustment(
            deps.as_ref().storage,
            strategy,
            env.block.time.plus_seconds(25 * 60 * 60),
        )
        .unwrap();

        assert_eq!(adjustment, Decimal::one());
    }
}
