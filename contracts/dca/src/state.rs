use cosmwasm_std::Addr;
use cosmwasm_std::{Api, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use base::executions::dca_execution::DCAExecutionInformation;
use base::executions::execution::Execution;
use base::pair::Pair;
use base::triggers::price_trigger::PriceTrigger;
use base::triggers::time_trigger::TimeTrigger;
use base::triggers::trigger::Trigger;
use base::vaults::dca_vault::DCAConfiguration;
use base::vaults::vault::Vault;

use crate::msg::{InstantiateMsg, MigrateMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cache {
    pub vault_id: Uint128,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub vault_count: Uint128,
    pub trigger_count: Uint128,
}

impl Config {
    pub fn validate(&self, api: &dyn Api) -> Result<Addr, cosmwasm_std::StdError> {
        api.addr_validate(self.admin.as_ref())
    }
}

impl From<InstantiateMsg> for Config {
    fn from(msg: InstantiateMsg) -> Self {
        Config {
            admin: Addr::unchecked(msg.admin),
            vault_count: Uint128::zero(),
            trigger_count: Uint128::zero(),
        }
    }
}

impl From<MigrateMsg> for Config {
    fn from(msg: MigrateMsg) -> Self {
        Config {
            admin: Addr::unchecked(msg.admin),
            vault_count: Uint128::zero(),
            trigger_count: Uint128::zero(),
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config_v1");

pub const CACHE: Item<Cache> = Item::new("cache_v1");

pub const PAIRS: Map<Addr, Pair> = Map::new("pairs_v1");

pub const ACTIVE_VAULTS: Map<(Addr, u128), Vault<DCAConfiguration>> = Map::new("active_vaults_v1");
pub const INACTIVE_VAULTS: Map<(Addr, u128), Vault<DCAConfiguration>> =
    Map::new("inactive_vaults_v1");
pub const CANCELLED_VAULTS: Map<(Addr, u128), Vault<DCAConfiguration>> =
    Map::new("cancelled_vaults_v1");

pub const TIME_TRIGGERS: Map<u128, Trigger<TimeTrigger>> = Map::new("time_triggers_v1");

pub const PRICE_TRIGGERS: Map<u128, Trigger<PriceTrigger>> = Map::new("price_triggers_v1");
pub const PRICE_OR_HIGHER: Map<(Addr, String), Vec<u128>> =
    Map::new("price_or_higher_v1");
pub const PRICE_OR_LOWER: Map<(Addr, String), Vec<u128>> =
    Map::new("price_or_lower_v1");


pub const EXECUTIONS: Map<u128, Vec<Execution<DCAExecutionInformation>>> = Map::new("execution_v1");
