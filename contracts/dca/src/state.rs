use base::events::event::Event;
use base::events::event::EventBuilder;
use cosmwasm_std::Addr;
use cosmwasm_std::StdResult;
use cosmwasm_std::Storage;
use cosmwasm_std::Uint128;
use cw_storage_plus::Index;
use cw_storage_plus::IndexList;
use cw_storage_plus::IndexedMap;
use cw_storage_plus::MultiIndex;
use cw_storage_plus::{Item, Map};
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

pub const FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID: Map<u128, FINLimitOrderConfiguration> =
    Map::new("fin_limit_order_configurations_by_vault_id_v1");

pub struct EventIndexes<'a> {
    pub address_resource_id_id_idx: MultiIndex<'a, (Addr, u128, u64), Event, u64>,
}

impl<'a> IndexList<Event> for EventIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Event>> + '_> {
        let v: Vec<&dyn Index<Event>> = vec![&self.address_resource_id_id_idx];
        Box::new(v.into_iter())
    }
}

pub fn event_store<'a>() -> IndexedMap<'a, u64, Event, EventIndexes<'a>> {
    let indexes = EventIndexes {
        address_resource_id_id_idx: MultiIndex::new(
            |_, e| (e.address.clone(), e.resource_id.u128(), e.id),
            "events_v1",
            "events_v1__address_resource_id_id_idx",
        ),
    };
    IndexedMap::new("events_v1", indexes)
}

const EVENT_COUNTER: Item<u64> = Item::new("events_v1_counter");

fn fetch_and_increment_counter(store: &mut dyn Storage, counter: Item<u64>) -> StdResult<u64> {
    let id = counter.may_load(store)?.unwrap_or_default() + 1;
    counter.save(store, &id)?;
    Ok(id)
}

pub fn save_event(store: &mut dyn Storage, event_builder: EventBuilder) -> StdResult<u64> {
    let event = event_builder.build(fetch_and_increment_counter(store, EVENT_COUNTER)?.into());
    event_store().save(store, event.id, &event.clone())?;
    Ok(event.id)
}
