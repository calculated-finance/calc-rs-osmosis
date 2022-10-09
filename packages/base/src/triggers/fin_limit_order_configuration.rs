use cosmwasm_std::{Addr, Decimal256, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::trigger::{Trigger, TriggerBuilder, TriggerVariant};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FINLimitOrderConfiguration {
    pub target_price: Decimal256,
}

impl From<FINLimitOrderConfiguration> for TriggerBuilder<FINLimitOrderConfiguration> {
    fn from(fin_limit_order_configuration: FINLimitOrderConfiguration) -> Self {
        TriggerBuilder {
            id: Uint128::zero(),
            owner: Addr::unchecked(""),
            variant: TriggerVariant::Time,
            vault_id: Uint128::zero(),
            configuration: fin_limit_order_configuration,
        }
    }
}

impl TriggerBuilder<FINLimitOrderConfiguration> {
    pub fn new_price_trigger() -> TriggerBuilder<FINLimitOrderConfiguration> {
        let fin_limit_order_configuration: FINLimitOrderConfiguration =
            FINLimitOrderConfiguration {
                target_price: Decimal256::zero(),
            };
        TriggerBuilder {
            id: Uint128::zero(),
            owner: Addr::unchecked(""),
            variant: TriggerVariant::Time,
            vault_id: Uint128::zero(),
            configuration: fin_limit_order_configuration,
        }
    }

    pub fn id(mut self, id: Uint128) -> TriggerBuilder<FINLimitOrderConfiguration> {
        self.id = id;
        self
    }

    pub fn owner(mut self, owner: Addr) -> TriggerBuilder<FINLimitOrderConfiguration> {
        self.owner = owner;
        self
    }

    pub fn vault_id(mut self, vault_id: Uint128) -> TriggerBuilder<FINLimitOrderConfiguration> {
        self.vault_id = vault_id;
        self
    }

    pub fn target_price(
        mut self,
        target_price: Decimal256,
    ) -> TriggerBuilder<FINLimitOrderConfiguration> {
        self.configuration.target_price = target_price;
        self
    }

    pub fn build(self) -> Trigger<FINLimitOrderConfiguration> {
        Trigger {
            id: self.id,
            owner: self.owner,
            variant: self.variant,
            vault_id: self.vault_id,
            configuration: self.configuration,
        }
    }
}
