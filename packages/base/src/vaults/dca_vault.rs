use cosmwasm_std::{Addr, Coin, Decimal256, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::vault::{Balance, Vault, VaultBuilder};
use crate::pair::Pair;
use crate::triggers::trigger::TriggerVariant;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DCAConfiguration {
    pub pair: Pair,
    pub swap_amount: Uint128,
    pub position_type: PositionType,
    pub slippage_tolerance: Option<Decimal256>,
}

impl Vault<DCAConfiguration> {
    // these functions can't be found in consuming project?
    pub fn get_swap_denom(&self) -> &String {
        let denom = if self.configuration.position_type == PositionType::Enter {
            &self.configuration.pair.quote_denom
        } else {
            &self.configuration.pair.base_denom
        };
        denom
    }

    pub fn get_current_balance(&self) -> &Coin {
        &self.balances[0].current
    }
}

impl VaultBuilder<DCAConfiguration> {
    pub fn new() -> VaultBuilder<DCAConfiguration> {
        let balance: Balance = Balance {
            starting: Coin {
                denom: "".to_string(),
                amount: Uint128::zero(),
            },
            current: Coin {
                denom: "".to_string(),
                amount: Uint128::zero(),
            },
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
            balances: vec![balance],
            configuration,
            trigger_id: Uint128::zero(),
            trigger_variant: TriggerVariant::FINLimitOrder,
        }
    }

    pub fn id(mut self, id: Uint128) -> VaultBuilder<DCAConfiguration> {
        self.id = id;
        self
    }

    pub fn owner(mut self, owner: Addr) -> VaultBuilder<DCAConfiguration> {
        self.owner = owner;
        self
    }

    pub fn balance(mut self, balance: Coin) -> VaultBuilder<DCAConfiguration> {
        self.balances = vec![Balance {
            starting: balance.clone(),
            current: balance,
        }];
        self
    }

    pub fn pair_address(mut self, address: Addr) -> VaultBuilder<DCAConfiguration> {
        self.configuration.pair.address = address;
        self
    }

    pub fn pair_base_denom(mut self, base_denom: String) -> VaultBuilder<DCAConfiguration> {
        self.configuration.pair.base_denom = base_denom;
        self
    }

    pub fn pair_quote_denom(mut self, quote_denom: String) -> VaultBuilder<DCAConfiguration> {
        self.configuration.pair.quote_denom = quote_denom;
        self
    }

    pub fn swap_amount(mut self, swap_amount: Uint128) -> VaultBuilder<DCAConfiguration> {
        self.configuration.swap_amount = swap_amount;
        self
    }

    pub fn slippage_tolerance(
        mut self,
        slippage_tolerance: Option<Decimal256>,
    ) -> VaultBuilder<DCAConfiguration> {
        self.configuration.slippage_tolerance = slippage_tolerance;
        self
    }

    pub fn position_type(mut self, position_type: PositionType) -> VaultBuilder<DCAConfiguration> {
        self.configuration.position_type = position_type;
        self
    }

    pub fn trigger_id(mut self, trigger_id: Uint128) -> VaultBuilder<DCAConfiguration> {
        self.trigger_id = trigger_id;
        self
    }

    pub fn trigger_variant(
        mut self,
        trigger_variant: TriggerVariant,
    ) -> VaultBuilder<DCAConfiguration> {
        self.trigger_variant = trigger_variant;
        self
    }

    pub fn build(self) -> Vault<DCAConfiguration> {
        Vault {
            id: self.id,
            owner: self.owner,
            balances: self.balances,
            configuration: self.configuration,
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
