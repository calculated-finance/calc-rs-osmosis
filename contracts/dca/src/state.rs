use crate::vault::Vault;
use crate::vault::VaultBuilder;
use base::events::event::Event;
use base::events::event::EventBuilder;
use base::pair::Pair;
use base::triggers::trigger::Trigger;
use base::triggers::trigger::TriggerConfiguration;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cosmwasm_std::StdResult;
use cosmwasm_std::Storage;
use cosmwasm_std::Uint128;
use cw_storage_plus::Bound;
use cw_storage_plus::Index;
use cw_storage_plus::IndexList;
use cw_storage_plus::IndexedMap;
use cw_storage_plus::MultiIndex;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Cache {
    pub vault_id: Uint128,
    pub owner: Addr,
}

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub fee_collector: Addr,
    pub fee_percent: Uint128,
    pub staking_router_address: Addr,
}

#[cw_serde]
pub struct LimitOrderCache {
    pub offer_amount: Uint128,
    pub original_offer_amount: Uint128,
    pub filled: Uint128,
}

#[cw_serde]
pub struct TimeTriggerCache {
    pub trigger_id: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config_v1");

pub const CACHE: Item<Cache> = Item::new("cache_v1");

pub const LIMIT_ORDER_CACHE: Item<LimitOrderCache> = Item::new("limit_order_cache_v1");

pub const PAIRS: Map<Addr, Pair> = Map::new("pairs_v1");

pub const VAULT_COUNTER: Item<u64> = Item::new("vault_counter_v1");

struct VaultIndexes<'a> {
    pub owner: MultiIndex<'a, (Addr, u128), Vault, u128>,
}

impl<'a> IndexList<Vault> for VaultIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Vault>> + '_> {
        let v: Vec<&dyn Index<Vault>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

fn vault_store<'a>() -> IndexedMap<'a, u128, Vault, VaultIndexes<'a>> {
    let indexes = VaultIndexes {
        owner: MultiIndex::new(
            |_, v| (v.owner.clone(), v.id.into()),
            "vaults_v5",
            "vaults_v5__owner",
        ),
    };
    IndexedMap::new("vaults_v5", indexes)
}

pub fn save_vault(store: &mut dyn Storage, vault_builder: VaultBuilder) -> StdResult<Vault> {
    let vault = vault_builder.build(fetch_and_increment_counter(store, VAULT_COUNTER)?.into());
    vault_store().save(store, vault.id.into(), &vault)?;
    Ok(vault)
}

pub fn get_vault(store: &dyn Storage, vault_id: Uint128) -> StdResult<Vault> {
    vault_store().load(store, vault_id.into())
}

pub fn get_vaults_by_address(
    store: &dyn Storage,
    address: Addr,
    start_after: Option<u128>,
    limit: Option<u8>,
) -> StdResult<Vec<Vault>> {
    Ok(vault_store()
        .idx
        .owner
        .sub_prefix(address)
        .range(
            store,
            start_after.map(|vault_id| Bound::exclusive((vault_id, vault_id))),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit.unwrap_or(30) as usize)
        .map(|result| result.expect("a vault stored by id").1)
        .collect::<Vec<Vault>>())
}

pub fn update_vault<T>(store: &mut dyn Storage, vault_id: Uint128, update_fn: T) -> StdResult<Vault>
where
    T: FnOnce(Option<Vault>) -> StdResult<Vault>,
{
    vault_store().update(store, vault_id.into(), update_fn)
}

pub fn clear_vaults(store: &mut dyn Storage) {
    vault_store().clear(store)
}

pub const TRIGGERS: Map<u128, Trigger> = Map::new("triggers_v1");

pub const TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX: Map<u128, u128> =
    Map::new("trigger_id_by_fin_limit_order_idx_v1");

pub const TRIGGER_IDS_BY_TARGET_TIME: Map<u64, Vec<u128>> =
    Map::new("trigger_ids_by_target_time_v1");

pub fn save_trigger(store: &mut dyn Storage, trigger: Trigger) -> StdResult<Uint128> {
    TRIGGERS.save(store, trigger.vault_id.into(), &trigger)?;
    match trigger.configuration {
        TriggerConfiguration::Time { target_time } => {
            let existing_triggers_at_time =
                TRIGGER_IDS_BY_TARGET_TIME.may_load(store, target_time.seconds())?;

            match existing_triggers_at_time {
                Some(_) => {
                    let mut triggers = existing_triggers_at_time.unwrap();
                    triggers.push(trigger.vault_id.into());
                    TRIGGER_IDS_BY_TARGET_TIME.save(store, target_time.seconds(), &triggers)?;
                }
                None => {
                    let mut triggers = Vec::new();
                    triggers.push(trigger.vault_id.into());
                    TRIGGER_IDS_BY_TARGET_TIME.save(store, target_time.seconds(), &triggers)?;
                }
            }
        }
        TriggerConfiguration::FINLimitOrder { order_idx, .. } => {
            if order_idx.is_some() {
                TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX.save(
                    store,
                    order_idx.unwrap().u128(),
                    &trigger.vault_id.into(),
                )?;
            }
        }
    }
    Ok(trigger.vault_id)
}

pub fn get_trigger(store: &dyn Storage, vault_id: Uint128) -> StdResult<Trigger> {
    TRIGGERS.load(store, vault_id.into())
}

pub fn delete_trigger(store: &mut dyn Storage, vault_id: Uint128) -> StdResult<Uint128> {
    let trigger = TRIGGERS.load(store, vault_id.into())?;
    TRIGGERS.remove(store, trigger.vault_id.into());
    match trigger.configuration {
        TriggerConfiguration::Time { target_time } => {
            let existing_triggers_at_time =
                TRIGGER_IDS_BY_TARGET_TIME.may_load(store, target_time.seconds())?;

            if existing_triggers_at_time.is_some() {
                let mut triggers = existing_triggers_at_time.unwrap();
                triggers.retain(|&t| t != vault_id.into());
                TRIGGER_IDS_BY_TARGET_TIME.save(store, target_time.seconds(), &triggers)?;
            }
        }
        TriggerConfiguration::FINLimitOrder { order_idx, .. } => {
            if order_idx.is_some() {
                TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX.remove(store, order_idx.unwrap().u128());
            }
        }
    }
    Ok(trigger.vault_id)
}

pub fn clear_triggers(store: &mut dyn Storage) {
    TRIGGERS.clear(store);
    TRIGGER_IDS_BY_TARGET_TIME.clear(store);
    TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX.clear(store);
}

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
            "events_v5",
            "events_v5__resource_id",
        ),
    };
    IndexedMap::new("events_v5", indexes)
}

fn fetch_and_increment_counter(store: &mut dyn Storage, counter: Item<u64>) -> StdResult<u64> {
    let id = counter.may_load(store)?.unwrap_or_default() + 1;
    counter.save(store, &id)?;
    Ok(id)
}

pub const EVENT_COUNTER: Item<u64> = Item::new("event_counter_v1");

pub fn create_event(store: &mut dyn Storage, event_builder: EventBuilder) -> StdResult<u64> {
    let event = event_builder.build(fetch_and_increment_counter(store, EVENT_COUNTER)?.into());
    event_store().save(store, event.id, &event.clone())?;
    Ok(event.id)
}
