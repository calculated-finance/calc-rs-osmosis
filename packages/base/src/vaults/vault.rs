use crate::pair::Pair;
use cosmwasm_std::{Addr, Coin, Decimal256, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    Enter,
    Exit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum VaultConfiguration {
    DCA {
        pair: Pair,
        swap_amount: Uint128,
        position_type: PositionType,
        slippage_tolerance: Option<Decimal256>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum VaultStatus {
    Active,
    Inactive,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vault {
    pub id: Uint128,
    pub created_at: Timestamp,
    pub owner: Addr,
    pub balances: Vec<Coin>,
    pub configuration: VaultConfiguration,
    pub status: VaultStatus,
    pub trigger_id: Option<Uint128>,
}

impl Vault {
    pub fn get_swap_denom(&self) -> String {
        match &self.configuration {
            VaultConfiguration::DCA {
                pair,
                swap_amount: _,
                position_type,
                slippage_tolerance: _,
            } => {
                if position_type.to_owned() == PositionType::Enter {
                    return pair.quote_denom.clone();
                }
                pair.base_denom.clone()
            }
        }
    }

    pub fn get_receive_denom(&self) -> String {
        match &self.configuration {
            VaultConfiguration::DCA {
                pair,
                swap_amount: _,
                position_type,
                slippage_tolerance: _,
            } => {
                if position_type.to_owned() == PositionType::Enter {
                    return pair.base_denom.clone();
                }

                pair.quote_denom.clone()
            }
        }
    }

    pub fn get_current_balance(&self) -> Coin {
        self.balances[0].clone()
    }

    pub fn get_swap_amount(&self) -> Coin {
        if self.low_funds() {
            self.balances[0].clone()
        } else {
            match &self.configuration {
                VaultConfiguration::DCA {
                    pair: _,
                    swap_amount,
                    position_type: _,
                    slippage_tolerance: _,
                } => Coin {
                    denom: self.get_swap_denom(),
                    amount: swap_amount.clone(),
                },
            }
        }
    }

    pub fn low_funds(&self) -> bool {
        match self.configuration {
            VaultConfiguration::DCA {
                pair: _,
                swap_amount,
                position_type: _,
                slippage_tolerance: _,
            } => self.balances[0].amount < swap_amount,
        }
    }
}
