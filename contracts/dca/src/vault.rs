use base::{
    pair::Pair,
    triggers::trigger::TimeInterval,
    vaults::vault::{PositionType, VaultStatus},
};
use cosmwasm_std::{Addr, Coin, Decimal256, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vault {
    pub id: Uint128,
    pub created_at: Timestamp,
    pub owner: Addr,
    pub status: VaultStatus,
    pub balance: Coin,
    pub pair: Pair,
    pub swap_amount: Uint128,
    pub position_type: PositionType,
    pub slippage_tolerance: Option<Decimal256>,
    pub time_interval: TimeInterval,
}

impl Vault {
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
        Coin {
            denom: self.get_swap_denom(),
            amount: match self.low_funds() {
                true => self.balance.amount,
                false => self.swap_amount,
            },
        }
    }

    pub fn low_funds(&self) -> bool {
        self.balance.amount < self.swap_amount
    }
}
