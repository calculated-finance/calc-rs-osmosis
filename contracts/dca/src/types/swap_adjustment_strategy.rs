use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Uint128};

#[cw_serde]
pub enum SwapAdjustmentStrategy {
    DcaPlus {
        escrow_level: Decimal,
        model_id: u8,
        total_deposit: Coin,
        standard_dca_swapped_amount: Coin,
        standard_dca_received_amount: Coin,
        escrowed_balance: Coin,
    },
}

impl SwapAdjustmentStrategy {
    pub fn dca_plus_model_id(&self) -> u8 {
        match self {
            SwapAdjustmentStrategy::DcaPlus { model_id, .. } => *model_id,
        }
    }

    pub fn dca_plus_total_deposit(&self) -> Coin {
        match self {
            SwapAdjustmentStrategy::DcaPlus { total_deposit, .. } => total_deposit.clone(),
        }
    }

    pub fn dca_plus_escrowed_balance(&self) -> Coin {
        match self {
            SwapAdjustmentStrategy::DcaPlus {
                escrowed_balance, ..
            } => escrowed_balance.clone(),
        }
    }

    pub fn dca_plus_standard_dca_swapped_amount(&self) -> Coin {
        match self {
            SwapAdjustmentStrategy::DcaPlus {
                standard_dca_swapped_amount,
                ..
            } => standard_dca_swapped_amount.clone(),
        }
    }

    pub fn dca_plus_standard_dca_received_amount(&self) -> Coin {
        match self {
            SwapAdjustmentStrategy::DcaPlus {
                standard_dca_received_amount,
                ..
            } => standard_dca_received_amount.clone(),
        }
    }

    pub fn dca_plus_escrow_level(&self) -> Decimal {
        match self {
            SwapAdjustmentStrategy::DcaPlus { escrow_level, .. } => *escrow_level,
        }
    }

    pub fn can_continue(&self) -> bool {
        match self {
            SwapAdjustmentStrategy::DcaPlus {
                total_deposit,
                standard_dca_swapped_amount,
                ..
            } => (total_deposit.amount - standard_dca_swapped_amount.amount) > Uint128::zero(),
        }
    }

    pub fn dca_plus_standard_dca_balance(self) -> Coin {
        if self.dca_plus_standard_dca_swapped_amount().amount
            >= self.dca_plus_total_deposit().amount
        {
            return Coin::new(0, self.dca_plus_standard_dca_swapped_amount().denom);
        }

        Coin::new(
            (self.dca_plus_total_deposit().amount
                - self.dca_plus_standard_dca_swapped_amount().amount)
                .into(),
            self.dca_plus_standard_dca_swapped_amount().denom,
        )
    }
}
