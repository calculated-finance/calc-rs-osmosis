use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Copy)]
pub struct DcaPlusConfig {
    pub escrow_level: Decimal,
    pub model_id: u8,
    pub total_deposit: Uint128,
    pub standard_dca_swapped_amount: Uint128,
    pub standard_dca_received_amount: Uint128,
    pub escrowed_balance: Uint128,
}

impl DcaPlusConfig {
    pub fn has_sufficient_funds(self) -> bool {
        self.total_deposit - self.standard_dca_swapped_amount > Uint128::new(50000)
    }
}
