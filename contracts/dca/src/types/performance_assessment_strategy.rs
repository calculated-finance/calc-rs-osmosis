use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

use super::vault::Vault;

#[cw_serde]
pub enum PerformanceAssessmentStrategy {
    CompareToStandardDca {
        swapped_amount: Coin,
        received_amount: Coin,
    },
}

#[cw_serde]
pub enum PerformanceAssessmentStrategyParams {
    CompareToStandardDca,
}

impl PerformanceAssessmentStrategy {
    pub fn should_continue(&self, vault: &Vault) -> bool {
        match self {
            PerformanceAssessmentStrategy::CompareToStandardDca { swapped_amount, .. } => {
                vault.deposited_amount.amount > swapped_amount.amount
            }
        }
    }
}
