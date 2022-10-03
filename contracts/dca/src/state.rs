use cosmwasm_std::Addr;
use cosmwasm_std::{Api, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use base::executions::dca_execution::DCAExecutionInformation;
use base::executions::execution::Execution;
use base::pair::Pair;
use base::triggers::fin_limit_order_trigger::FINPriceTrigger;
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
pub const FIN_PRICE_TRIGGERS: Map<u128, Trigger<FINPriceTrigger>> =
    Map::new("fin_price_triggers_v1");
pub const FIN_PRICE_TRIGGERS_BY_ORDER_ID: Map<u128, u128> =
    Map::new("fin_price_triggers_by_order_id_v1"); // order_idx -> trigger_id -> trigger -> vault

pub const EXECUTIONS: Map<u128, Vec<Execution<DCAExecutionInformation>>> = Map::new("execution_v1");
