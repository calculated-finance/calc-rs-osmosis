use base::events::dca_event::DCAEventInfo;
use base::events::event::Event;
use cosmwasm_std::Addr;
use cosmwasm_std::Uint128;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use base::pair::Pair;
use base::triggers::fin_limit_order_configuration::FINLimitOrderConfiguration;
use base::triggers::time_configuration::TimeConfiguration;
use base::triggers::trigger::Trigger;
use base::vaults::dca_vault::{DCAConfiguration, DCAStatus};
use base::vaults::vault::Vault;

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

pub struct FINLimitOrderTriggerIndexes<'a> {
    pub order_idx: UniqueIndex<'a, u128, Trigger<FINLimitOrderConfiguration>, u128>,
}

impl<'a> IndexList<Trigger<FINLimitOrderConfiguration>> for FINLimitOrderTriggerIndexes<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn Index<Trigger<FINLimitOrderConfiguration>>> + '_> {
        let v: Vec<&dyn Index<Trigger<FINLimitOrderConfiguration>>> = vec![&self.order_idx];
        Box::new(v.into_iter())
    }
}

pub fn fin_limit_order_triggers<'a>(
) -> IndexedMap<'a, u128, Trigger<FINLimitOrderConfiguration>, FINLimitOrderTriggerIndexes<'a>> {
    let indexes = FINLimitOrderTriggerIndexes {
        order_idx: UniqueIndex::new(
            |d| u128::from(d.configuration.order_idx),
            "fin_limit_order_triggers_order_idx_v1",
        ),
    };
    IndexedMap::new("fin_limit_order_triggers_v1", indexes)
}

pub const CONFIG: Item<Config> = Item::new("config_v1");

pub const CACHE: Item<Cache> = Item::new("cache_v1");

pub const LIMIT_ORDER_CACHE: Item<LimitOrderCache> = Item::new("limit_order_cache_v1");

pub const PAIRS: Map<Addr, Pair> = Map::new("pairs_v1");

pub const VAULTS: Map<(Addr, u128), Vault<DCAConfiguration, DCAStatus>> = Map::new("vaults_v1");

pub const TIME_TRIGGERS: Map<u128, Trigger<TimeConfiguration>> = Map::new("time_triggers_v1");

pub const TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID: Map<u128, TimeConfiguration> =
    Map::new("time_trigger_configurations_by_vault_id_v1");

pub const FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID: Map<u128, FINLimitOrderConfiguration> =
    Map::new("fin_limit_order_configurations_by_vault_id_v1");

pub const EVENTS: Map<u128, Vec<Event<DCAEventInfo>>> = Map::new("events_v1");
