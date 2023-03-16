use cosmwasm_std::{Coin, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DcaPlusConfig {
    pub escrow_level: Decimal,
    pub model_id: u8,
    pub total_deposit: Coin,
    pub standard_dca_swapped_amount: Coin,
    pub standard_dca_received_amount: Coin,
    pub escrowed_balance: Coin,
}

impl DcaPlusConfig {
    pub fn new(
        escrow_level: Decimal,
        model_id: u8,
        total_deposit: Coin,
        receive_denom: String,
    ) -> Self {
        Self {
            escrow_level,
            model_id,
            total_deposit: total_deposit.clone(),
            standard_dca_swapped_amount: Coin::new(0, total_deposit.denom),
            standard_dca_received_amount: Coin::new(0, receive_denom.clone()),
            escrowed_balance: Coin::new(0, receive_denom),
        }
    }

    pub fn standard_dca_balance(self) -> Coin {
        if self.standard_dca_swapped_amount.amount >= self.total_deposit.amount {
            return Coin::new(0, self.standard_dca_swapped_amount.denom);
        }

        Coin::new(
            (self.total_deposit.amount - self.standard_dca_swapped_amount.amount).into(),
            self.standard_dca_swapped_amount.denom,
        )
    }

    pub fn has_sufficient_funds(self) -> bool {
        self.standard_dca_balance().amount > Uint128::new(50000)
    }
}
