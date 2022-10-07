use cosmwasm_std::Addr;
use cosmwasm_std::{Api, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use base::executions::dca_execution::DCAExecutionInformation;
use base::executions::execution::Execution;
use base::pair::Pair;
use base::triggers::fin_limit_order_configuration::FINLimitOrderConfiguration;
use base::triggers::time_configuration::TimeConfiguration;
use base::triggers::trigger::Trigger;
use base::vaults::dca_vault::{DCAConfiguration, DCAStatus};
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LimitOrderCache {
    pub offer_amount: Uint128,
    pub original_offer_amount: Uint128,
    pub filled: Uint128,
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

pub const LIMIT_ORDER_CACHE: Item<LimitOrderCache> = Item::new("limit_order_cache_v1");

pub const PAIRS: Map<Addr, Pair> = Map::new("pairs_v1");

pub const VAULTS: Map<(Addr, u128), Vault<DCAConfiguration, DCAStatus>> = Map::new("vaults_v1");

pub const TIME_TRIGGERS: Map<u128, Trigger<TimeConfiguration>> = Map::new("time_triggers_v1");
pub const TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID: Map<u128, TimeConfiguration> =
    Map::new("time_trigger_configurations_by_vault_id_v1");

pub const FIN_LIMIT_ORDER_TRIGGERS: Map<u128, Trigger<FINLimitOrderConfiguration>> =
    Map::new("fin_limit_order_triggers_v1");
pub const FIN_LIMIT_ORDER_TRIGGER_IDS_BY_ORDER_IDX: Map<u128, u128> =
    Map::new("fin_limit_order_trigger_ids_by_order_idx_v1");
pub const FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID: Map<u128, FINLimitOrderConfiguration> =
    Map::new("fin_limit_order_configurations_by_vault_id_v1");

pub const EXECUTIONS: Map<u128, Vec<Execution<DCAExecutionInformation>>> = Map::new("execution_v1");
