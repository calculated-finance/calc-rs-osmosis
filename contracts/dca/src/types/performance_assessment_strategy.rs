use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

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
    pub fn standard_dca_swapped_amount(&self) -> Coin {
        match self {
            PerformanceAssessmentStrategy::CompareToStandardDca { swapped_amount, .. } => {
                swapped_amount.clone()
            }
        }
    }

    pub fn standard_dca_received_amount(&self) -> Coin {
        match self {
            PerformanceAssessmentStrategy::CompareToStandardDca {
                received_amount, ..
            } => received_amount.clone(),
        }
    }

    pub fn standard_dca_balance(self, deposited_amount: Coin) -> Coin {
        if self.standard_dca_swapped_amount().amount >= deposited_amount.amount {
            return Coin::new(0, self.standard_dca_swapped_amount().denom);
        }

        Coin::new(
            (deposited_amount.amount - self.standard_dca_swapped_amount().amount).into(),
            self.standard_dca_swapped_amount().denom,
        )
    }
}
