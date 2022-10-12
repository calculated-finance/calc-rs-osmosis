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

pub struct VaultBuilder {
    pub id: Uint128,
    pub owner: Addr,
    pub created_at: Timestamp,
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

impl VaultBuilder {
    pub fn new(id: Uint128, owner: Addr, created_at: Timestamp) -> VaultBuilder {
        let balance: Coin = Coin {
            denom: "".to_string(),
            amount: Uint128::zero(),
        };
        let pair: Pair = Pair {
            address: Addr::unchecked(""),
            base_denom: "".to_string(),
            quote_denom: "".to_string(),
        };
        let configuration: VaultConfiguration = VaultConfiguration::DCA {
            pair,
            swap_amount: Uint128::zero(),
            position_type: PositionType::Enter,
            slippage_tolerance: None,
        };
        VaultBuilder {
            id,
            owner,
            created_at,
            balances: vec![balance],
            configuration,
            status: VaultStatus::Active,
            trigger_id: Some(Uint128::zero()),
        }
    }

    pub fn balance(mut self, balance: Coin) -> VaultBuilder {
        self.balances = vec![balance];
        self
    }

    pub fn configuration(mut self, configuration: VaultConfiguration) -> Self {
        self.configuration = configuration;
        self
    }

    pub fn status(mut self, status: VaultStatus) -> VaultBuilder {
        self.status = status;
        self
    }

    pub fn trigger_id(mut self, trigger_id: Uint128) -> VaultBuilder {
        self.trigger_id = Some(trigger_id);
        self
    }

    pub fn build(self) -> Vault {
        Vault {
            id: self.id,
            owner: self.owner,
            created_at: self.created_at,
            balances: self.balances,
            configuration: self.configuration,
            status: self.status,
            trigger_id: self.trigger_id,
        }
    }
}
