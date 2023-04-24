use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum SwapAdjustmentStrategy {
    DcaPlus { model_id: u8 },
}

#[cw_serde]
pub enum SwapAdjustmentStrategyParams {
    DcaPlus,
}

impl SwapAdjustmentStrategy {
    pub fn dca_plus_model_id(&self) -> u8 {
        match self {
            SwapAdjustmentStrategy::DcaPlus { model_id, .. } => *model_id,
        }
    }
}
