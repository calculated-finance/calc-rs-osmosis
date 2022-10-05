use cosmwasm_std::{Addr, Coin, Decimal256, Timestamp, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::vault::{Vault, VaultBuilder};
use crate::pair::Pair;
use crate::triggers::trigger::TriggerVariant;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DCAConfiguration {
    pub pair: Pair,
    pub swap_amount: Uint128,
    pub position_type: PositionType,
    pub slippage_tolerance: Option<Decimal256>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DCAStatus {
    Active,
    Inactive,
}

impl Vault<DCAConfiguration, DCAStatus> {
    pub fn get_swap_denom(&self) -> String {
        let denom = if self.configuration.position_type == PositionType::Enter {
            &self.configuration.pair.quote_denom
        } else {
            &self.configuration.pair.base_denom
        };
        denom.clone()
    }

    pub fn get_receive_denom(&self) -> String {
        let denom = if self.configuration.position_type == PositionType::Enter {
            &self.configuration.pair.base_denom
        } else {
            &self.configuration.pair.quote_denom
        };
        denom.clone()
    }

    pub fn get_current_balance(&self) -> Coin {
        self.balances[0].clone()
    }

    pub fn get_swap_amount(&self) -> Coin {
        if self.low_funds() {
            self.balances[0].clone()
        } else {
            Coin {
                denom: self.get_swap_denom().clone(),
                amount: self.configuration.swap_amount.clone(),
            }
        }
    }

    pub fn low_funds(&self) -> bool {
        self.configuration.swap_amount >= self.balances[0].amount
    }
}

impl VaultBuilder<DCAConfiguration, DCAStatus> {
    pub fn new() -> VaultBuilder<DCAConfiguration, DCAStatus> {
        let balance: Coin = Coin {
            denom: "".to_string(),
            amount: Uint128::zero(),
        };
        let pair: Pair = Pair {
            address: Addr::unchecked(""),
            base_denom: "".to_string(),
            quote_denom: "".to_string(),
        };
        let configuration: DCAConfiguration = DCAConfiguration {
            pair,
            swap_amount: Uint128::zero(),
            position_type: PositionType::Enter,
            slippage_tolerance: None,
        };
        VaultBuilder {
            id: Uint128::zero(),
            owner: Addr::unchecked(""),
            created_at: Timestamp::from_seconds(0u64),
            balances: vec![balance],
            configuration,
            status: DCAStatus::Active,
            trigger_id: Uint128::zero(),
            trigger_variant: TriggerVariant::FINLimitOrder,
        }
    }

    pub fn id(mut self, id: Uint128) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.id = id;
        self
    }

    pub fn owner(mut self, owner: Addr) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.owner = owner;
        self
    }

    pub fn created_at(
        mut self,
        created_at: Timestamp,
    ) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.created_at = created_at;
        self
    }

    pub fn balance(mut self, balance: Coin) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.balances = vec![balance];
        self
    }

    pub fn pair_address(mut self, address: Addr) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.configuration.pair.address = address;
        self
    }

    pub fn pair_base_denom(
        mut self,
        base_denom: String,
    ) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.configuration.pair.base_denom = base_denom;
        self
    }

    pub fn pair_quote_denom(
        mut self,
        quote_denom: String,
    ) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.configuration.pair.quote_denom = quote_denom;
        self
    }

    pub fn swap_amount(
        mut self,
        swap_amount: Uint128,
    ) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.configuration.swap_amount = swap_amount;
        self
    }

    pub fn slippage_tolerance(
        mut self,
        slippage_tolerance: Option<Decimal256>,
    ) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.configuration.slippage_tolerance = slippage_tolerance;
        self
    }

    pub fn position_type(
        mut self,
        position_type: PositionType,
    ) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.configuration.position_type = position_type;
        self
    }

    pub fn status(mut self, status: DCAStatus) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.status = status;
        self
    }

    pub fn trigger_id(mut self, trigger_id: Uint128) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.trigger_id = trigger_id;
        self
    }

    pub fn trigger_variant(
        mut self,
        trigger_variant: TriggerVariant,
    ) -> VaultBuilder<DCAConfiguration, DCAStatus> {
        self.trigger_variant = trigger_variant;
        self
    }

    pub fn build(self) -> Vault<DCAConfiguration, DCAStatus> {
        Vault {
            id: self.id,
            owner: self.owner,
            created_at: self.created_at,
            balances: self.balances,
            configuration: self.configuration,
            status: self.status,
            trigger_id: self.trigger_id,
            trigger_variant: self.trigger_variant,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    Enter,
    Exit,
}
