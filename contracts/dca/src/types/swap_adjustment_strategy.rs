use super::position_type::PositionType;
use crate::util::calculate_hash;
use cosmwasm_schema::cw_serde;

#[cw_serde]
#[derive(Hash)]
pub enum SwapAdjustmentStrategy {
    RiskWeightedAverage {
        model_id: u8,
        base_denom: BaseDenom,
        position_type: PositionType,
    },
}

#[cw_serde]
pub enum SwapAdjustmentStrategyParams {
    RiskWeightedAverage { base_denom: BaseDenom },
}

#[cw_serde]
#[derive(Hash)]
pub enum BaseDenom {
    Bitcoin,
}

impl SwapAdjustmentStrategy {
    pub fn hash(&self) -> u64 {
        calculate_hash(self)
    }

    pub fn ttl(&self) -> u64 {
        match self {
            SwapAdjustmentStrategy::RiskWeightedAverage { .. } => 60 * 60 * 25,
        }
    }

    pub fn risk_weighted_average_model_id(&self) -> u8 {
        match self {
            SwapAdjustmentStrategy::RiskWeightedAverage { model_id, .. } => *model_id,
        }
    }
}
