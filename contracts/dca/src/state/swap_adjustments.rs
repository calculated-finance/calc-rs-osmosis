use crate::types::swap_adjustment_strategy::SwapAdjustmentStrategy;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdResult, Storage, Timestamp};
use cw_storage_plus::Map;

#[cw_serde]
struct SwapAdjustment {
    value: Decimal,
    timestamp: u64,
}

const SWAP_ADJUSTMENTS: Map<u64, SwapAdjustment> = Map::new("buy_adjustments_v8");

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
            value,
            timestamp: block_time.seconds(),
        },
    )
}

pub fn get_swap_adjustment(
    storage: &dyn Storage,
    strategy: SwapAdjustmentStrategy,
    block_time: Timestamp,
) -> Decimal {
    let adjustment = SWAP_ADJUSTMENTS
        .load(storage, strategy.hash())
        .unwrap_or_else(|_| SwapAdjustment {
            value: Decimal::one(),
            timestamp: block_time.seconds(),
        });

    if adjustment.timestamp + strategy.ttl() > block_time.seconds() {
        adjustment.value
    } else {
        Decimal::one()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{position_type::PositionType, swap_adjustment_strategy::BaseDenom};
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Decimal,
    };

    #[test]
    fn gets_swap_adjustment_if_updated_within_ttl() {
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
            env.block.time.plus_seconds(1),
        );

        assert_eq!(adjustment, adjustment_value);
    }

    #[test]
    fn gets_default_swap_adjustment_if_not_updated_within_ttl() {
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
            strategy.clone(),
            env.block.time.plus_seconds(strategy.ttl() + 1),
        );

        assert_eq!(adjustment, Decimal::one());
    }
}
