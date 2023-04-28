use super::position_type::PositionType;
use crate::util::calculate_hash;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Decimal, Uint128};

#[cw_serde]
pub enum SwapAdjustmentStrategy {
    RiskWeightedAverage {
        model_id: u8,
        base_denom: BaseDenom,
        position_type: PositionType,
    },
    WeightedScale {
        base_receive_amount: Uint128,
        multiplier: Decimal,
        increase_only: bool,
    },
}

#[cw_serde]
pub enum SwapAdjustmentStrategyParams {
    RiskWeightedAverage {
        base_denom: BaseDenom,
    },
    WeightedScale {
        base_receive_amount: Uint128,
        multiplier: Decimal,
        increase_only: bool,
    },
}

#[cw_serde]
pub enum BaseDenom {
    Bitcoin,
}

impl SwapAdjustmentStrategy {
    pub fn hash(&self) -> u64 {
        calculate_hash(&to_binary(self).unwrap())
    }

    pub fn ttl(&self) -> u64 {
        match self {
            SwapAdjustmentStrategy::RiskWeightedAverage { .. } => 60 * 60 * 25,
            _ => 0,
        }
    }
}
