use cosmwasm_std::{Decimal256, Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::trigger::{Trigger, TriggerBuilder, TriggerVariant};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceTrigger {
    pub target_price: Decimal256,
    pub comparison_type: ComparisonType
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonType {
    EqualOrHigher,
    EqualOrLower
}

impl TriggerBuilder<PriceTrigger> {
    pub fn new_price_trigger() -> TriggerBuilder<PriceTrigger> {
        let price_trigger_configuration: PriceTrigger = PriceTrigger {
            comparison_type: ComparisonType::EqualOrHigher,
            target_price: Decimal256::zero()
        };
        TriggerBuilder {
            id: Uint128::zero(),
            owner: Addr::unchecked(""),
            variant: TriggerVariant::Time,
            vault_id: Uint128::zero(),
            configuration: price_trigger_configuration,
        }
    }

    pub fn id(mut self, id: Uint128) -> TriggerBuilder<PriceTrigger> {
        self.id = id;
        self
    }

    pub fn owner(mut self, owner: Addr) -> TriggerBuilder<PriceTrigger> {
        self.owner = owner;
        self
    }

    pub fn vault_id(mut self, vault_id: Uint128) -> TriggerBuilder<PriceTrigger> {
        self.vault_id = vault_id;
        self
    }

    pub fn target_price(mut self, target_price: Decimal256) -> TriggerBuilder<PriceTrigger> {
        self.configuration.target_price = target_price;
        self
    }

    pub fn comparison_type(mut self, comparison_type: ComparisonType) -> TriggerBuilder<PriceTrigger> {
        self.configuration.comparison_type = comparison_type;
        self
    }

    pub fn build(self) -> Trigger<PriceTrigger> {
        Trigger {
            id: self.id,
            owner: self.owner,
            variant: self.variant,
            vault_id: self.vault_id,
            configuration: self.configuration,
        }
    }
}
