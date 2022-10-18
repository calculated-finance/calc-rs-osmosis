use crate::vault::Vault;
use base::events::event::Event;
use base::events::event::EventBuilder;
use base::pair::Pair;
use base::triggers::trigger::Trigger;
use base::triggers::trigger::TriggerBuilder;
use base::triggers::trigger::TriggerConfiguration;
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cache {
    pub vault_id: Uint128,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub vault_count: Uint128,
    pub fee_collector: Addr,
    pub fee_percent: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LimitOrderCache {
    pub trigger_id: Uint128,
    pub offer_amount: Uint128,
    pub original_offer_amount: Uint128,
    pub filled: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TimeTriggerCache {
    pub trigger_id: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config_v1");

pub const CACHE: Item<Cache> = Item::new("cache_v1");

pub const LIMIT_ORDER_CACHE: Item<LimitOrderCache> = Item::new("limit_order_cache_v1");

pub const TIME_TRIGGER_CACHE: Item<TimeTriggerCache> = Item::new("time_trigger_cache_v1");

pub const PAIRS: Map<Addr, Pair> = Map::new("pairs_v1");

pub struct VaultIndexes<'a> {
    pub owner: MultiIndex<'a, (Addr, u128), Vault, u128>,
}

impl<'a> IndexList<Vault> for VaultIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Vault>> + '_> {
        let v: Vec<&dyn Index<Vault>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn vault_store<'a>() -> IndexedMap<'a, u128, Vault, VaultIndexes<'a>> {
    let indexes = VaultIndexes {
        owner: MultiIndex::new(
            |_, v| (v.owner.clone(), v.id.into()),
            "vaults_v1",
            "vaults_v1__owner",
        ),
    };
    IndexedMap::new("vaults_v1", indexes)
}

pub struct TriggerIndexes<'a> {
    pub vault_id: MultiIndex<'a, u128, Trigger, u128>,
    pub order_idx: MultiIndex<'a, u128, Trigger, u128>,
    pub variant: MultiIndex<'a, u8, Trigger, u128>,
}

impl<'a> IndexList<Trigger> for TriggerIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Trigger>> + '_> {
        let v: Vec<&dyn Index<Trigger>> = vec![&self.vault_id, &self.order_idx, &self.variant];
        Box::new(v.into_iter())
    }
}

pub fn trigger_store<'a>() -> IndexedMap<'a, u128, Trigger, TriggerIndexes<'a>> {
    let indexes = TriggerIndexes {
        vault_id: MultiIndex::new(
            |_, t| t.vault_id.u128(),
            "triggers_v1",
            "triggers_v1__vault_id",
        ),
        order_idx: MultiIndex::new(
            |_, t| match t.configuration {
                TriggerConfiguration::Time { .. } => Uint128::zero().u128(),
                TriggerConfiguration::FINLimitOrder { order_idx, .. } => {
                    order_idx.unwrap_or(Uint128::zero()).u128()
                }
            },
            "triggers_v1",
            "triggers_v1__order_idx",
        ),
        variant: MultiIndex::new(
            |_, t| match t.configuration {
                TriggerConfiguration::Time { .. } => 0,
                TriggerConfiguration::FINLimitOrder { .. } => 1,
            },
            "triggers_v1",
            "triggers_v1__variant",
        ),
    };
    IndexedMap::new("triggers_v1", indexes)
}

const TRIGGER_COUNTER: Item<u64> = Item::new("trigger_counter_v1");

pub fn create_trigger(
    store: &mut dyn Storage,
    trigger_builder: TriggerBuilder,
) -> StdResult<Uint128> {
    let trigger =
        trigger_builder.build(fetch_and_increment_counter(store, TRIGGER_COUNTER)?.into());
    trigger_store().save(store, trigger.id.into(), &trigger)?;
    Ok(trigger.id)
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
            |_, e| (e.resource_id.into(), e.id),
            "events_v1",
            "events_v1__resource_id",
        ),
    };
    IndexedMap::new("events_v1", indexes)
}

fn fetch_and_increment_counter(store: &mut dyn Storage, counter: Item<u64>) -> StdResult<u64> {
    let id = counter.may_load(store)?.unwrap_or_default() + 1;
    counter.save(store, &id)?;
    Ok(id)
}

const EVENT_COUNTER: Item<u64> = Item::new("event_counter_v1");

pub fn create_event(store: &mut dyn Storage, event_builder: EventBuilder) -> StdResult<u64> {
    let event = event_builder.build(fetch_and_increment_counter(store, EVENT_COUNTER)?.into());
    event_store().save(store, event.id, &event.clone())?;
    Ok(event.id)
}
