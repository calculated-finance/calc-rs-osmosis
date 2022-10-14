use base::{pair::Pair, vaults::vault::PositionType};
use cosmwasm_std::{Coin, Decimal256, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DCAConfiguration {
    pub balance: Coin,
    pub pair: Pair,
    pub swap_amount: Uint128,
    pub position_type: PositionType,
    pub slippage_tolerance: Option<Decimal256>,
}

impl DCAConfiguration {
    pub fn get_swap_denom(&self) -> String {
        if self.position_type.to_owned() == PositionType::Enter {
            return self.pair.quote_denom.clone();
        }
        self.pair.base_denom.clone()
    }

    pub fn get_receive_denom(&self) -> String {
        if self.position_type.to_owned() == PositionType::Enter {
            return self.pair.base_denom.clone();
        }
        self.pair.quote_denom.clone()
    }

    pub fn get_swap_amount(&self) -> Coin {
        if self.low_funds() {
            self.balance.clone()
        } else {
            Coin {
                denom: self.get_swap_denom(),
                amount: self.swap_amount.clone(),
            }
        }
    }

    pub fn low_funds(&self) -> bool {
        self.balance.amount < self.swap_amount
    }
}
