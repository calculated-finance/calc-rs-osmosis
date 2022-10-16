use base::events::event::Event;
use base::events::event::EventBuilder;
use base::pair::Pair;
use base::triggers::trigger::Trigger;
use base::triggers::trigger::TriggerConfiguration;
use base::vaults::vault::Vault;
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

use crate::dca_configuration::DCAConfiguration;

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
    pub fee_collector: Addr,
    pub fee_percent: Uint128,
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

pub struct VaultIndexes<'a> {
    pub owner: MultiIndex<'a, (Addr, u128), Vault<DCAConfiguration>, u128>,
}

impl<'a> IndexList<Vault<DCAConfiguration>> for VaultIndexes<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn Index<Vault<DCAConfiguration>>> + '_> {
        let v: Vec<&dyn Index<Vault<DCAConfiguration>>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn vault_store<'a>() -> IndexedMap<'a, u128, Vault<DCAConfiguration>, VaultIndexes<'a>> {
    let indexes = VaultIndexes {
        owner: MultiIndex::new(
            |_, v| (v.owner.clone(), v.id.u128()),
            "vaults_v1",
            "vaults_v1__variant",
        ),
    };
    IndexedMap::new("vaults_v1", indexes)
}

pub struct TriggerIndexes<'a> {
    pub variant: MultiIndex<'a, u8, Trigger, Uint128>,
}

impl<'a> IndexList<Trigger> for TriggerIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Trigger>> + '_> {
        let v: Vec<&dyn Index<Trigger>> = vec![&self.variant];
        Box::new(v.into_iter())
    }
}

pub fn trigger_store<'a>() -> IndexedMap<'a, u128, Trigger, TriggerIndexes<'a>> {
    let indexes = TriggerIndexes {
        variant: MultiIndex::new(
            |_, e| match e.configuration {
                TriggerConfiguration::Time {
                    time_interval: _,
                    target_time: _,
                } => 0,
                TriggerConfiguration::FINLimitOrder {
                    target_price: _,
                    order_idx: _,
                } => 1,
            },
            "triggers_v1",
            "triggers_v1__variant",
        ),
    };
    IndexedMap::new("triggers_v1", indexes)
}

pub const TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID: Map<u128, TriggerConfiguration> =
    Map::new("time_trigger_configurations_by_vault_id_v1");

pub const FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID: Map<u128, TriggerConfiguration> =
    Map::new("fin_limit_order_configurations_by_vault_id_v1");

pub struct EventIndexes<'a> {
    pub resource_id: MultiIndex<'a, (u128, u64), Event, u64>,
}

impl<'a> IndexList<Event> for EventIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Event>> + '_> {
        let v: Vec<&dyn Index<Event>> = vec![&self.resource_id];
        Box::new(v.into_iter())
    }
}

pub fn event_store<'a>() -> IndexedMap<'a, u64, Event, EventIndexes<'a>> {
    let indexes = EventIndexes {
        resource_id: MultiIndex::new(
            |_, e| (e.resource_id.u128(), e.id),
            "events_v1",
            "events_v1__resource_id",
        ),
    };
    IndexedMap::new("events_v1", indexes)
}

const EVENT_COUNTER: Item<u64> = Item::new("event_counter_v1");

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
