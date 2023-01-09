use super::state_helpers::fetch_and_increment_counter;
use base::events::event::{Event, EventBuilder};
use cosmwasm_std::{from_binary, to_binary, Binary, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, UniqueIndex};

const EVENT_COUNTER: Item<u64> = Item::new("event_counter_v20");

pub struct EventIndexes<'a> {
    pub resource_id: UniqueIndex<'a, (u128, u64), Binary, u64>,
}

impl<'a> IndexList<Binary> for EventIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Binary>> + '_> {
        let v: Vec<&dyn Index<Binary>> = vec![&self.resource_id];
        Box::new(v.into_iter())
    }
}

pub fn event_store<'a>() -> IndexedMap<'a, u64, Binary, EventIndexes<'a>> {
    let indexes = EventIndexes {
        resource_id: UniqueIndex::new(
            |event| {
                from_binary(&event)
                    .map(|event: Event| (event.resource_id.into(), event.id))
                    .expect("deserialised event")
            },
            "serialised_events_v20__resource_id",
        ),
    };
    IndexedMap::new("serialised_events_v20", indexes)
}

pub fn create_event(store: &mut dyn Storage, event_builder: EventBuilder) -> StdResult<u64> {
    let event = event_builder.build(fetch_and_increment_counter(store, EVENT_COUNTER)?.into());
    event_store().save(
        store,
        event.id,
        &to_binary(&event).expect("serialised event"),
    )?;
    Ok(event.id)
}

pub fn create_events(store: &mut dyn Storage, event_builders: Vec<EventBuilder>) -> StdResult<()> {
    for event_builder in event_builders {
        create_event(store, event_builder)?;
    }
    Ok(())
}

pub fn clear_events(store: &mut dyn Storage) {
    event_store().clear(store);
    EVENT_COUNTER.remove(store)
}
