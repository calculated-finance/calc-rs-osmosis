use super::state_helpers::fetch_and_increment_counter;
use crate::error::ContractError;
use base::events::event::{Event, EventBuilder};
use cosmwasm_std::{from_binary, to_binary, Binary, Response, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, UniqueIndex};

const EVENT_COUNTER: Item<u64> = Item::new("event_counter_v20");

pub struct EventIndexes<'a> {
    pub resource_id: UniqueIndex<'a, (u128, u64), Event, u64>,
}

pub struct SerialisedEventIndexes<'a> {
    pub resource_id: UniqueIndex<'a, (u128, u64), Binary, u64>,
}

impl<'a> IndexList<Event> for EventIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Event>> + '_> {
        let v: Vec<&dyn Index<Event>> = vec![&self.resource_id];
        Box::new(v.into_iter())
    }
}

impl<'a> IndexList<Binary> for SerialisedEventIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Binary>> + '_> {
        let v: Vec<&dyn Index<Binary>> = vec![&self.resource_id];
        Box::new(v.into_iter())
    }
}

pub fn event_store<'a>() -> IndexedMap<'a, u64, Event, EventIndexes<'a>> {
    let indexes = EventIndexes {
        resource_id: UniqueIndex::new(|e| (e.resource_id.into(), e.id), "events_v20__resource_id"),
    };
    IndexedMap::new("events_v20", indexes)
}

pub fn serialised_event_store<'a>() -> IndexedMap<'a, u64, Binary, SerialisedEventIndexes<'a>> {
    let indexes = SerialisedEventIndexes {
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
    event_store().save(store, event.id, &event.clone())?;
    serialised_event_store().save(
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

pub fn migrate_previous_events(
    store: &mut dyn Storage,
    limit: &mut u64,
) -> Result<Response, ContractError> {
    let mut event_to_migrate_id = serialised_event_store()
        .range(store, None, None, cosmwasm_std::Order::Ascending)
        .take(1 as usize)
        .map(|result| from_binary(&result.unwrap().1).unwrap())
        .collect::<Vec<Event>>()
        .first()
        .expect("earliest migrated event id")
        .id
        - 1;

    while event_to_migrate_id > 0 && limit > &mut 0 {
        let event_to_migrate = event_store().load(store, event_to_migrate_id)?;

        serialised_event_store().save(
            store,
            event_to_migrate.id,
            &to_binary(&event_to_migrate).expect("serialised event"),
        )?;

        event_to_migrate_id -= 1;
        *limit -= 1;
    }

    if event_to_migrate_id == 0 {
        return Err(ContractError::CustomError {
            val: "All events have been migrated".to_string(),
        });
    }

    return Ok(
        Response::new().add_attribute("last_event_migrated_id", event_to_migrate_id.to_string())
    );
}

#[cfg(test)]
mod event_migration_tests {
    use base::events::event::EventData;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Order, Uint128,
    };

    use super::*;

    #[test]
    fn should_save_events_properly() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let event_builder = EventBuilder::new(
            Uint128::new(1),
            env.block.clone(),
            EventData::DcaVaultCreated {},
        );

        create_event(deps.as_mut().storage, event_builder).unwrap();

        let old_event = event_store().load(deps.as_ref().storage, 1).unwrap();
        let new_event: Event = from_binary(
            &serialised_event_store()
                .load(deps.as_ref().storage, 1)
                .unwrap(),
        )
        .unwrap();

        assert_eq!(old_event, new_event);
    }

    #[test]
    fn should_migrate_the_previous_events_until_empty() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut event_count = 1;

        while event_count <= 10 {
            event_store()
                .save(
                    deps.as_mut().storage,
                    event_count,
                    &EventBuilder::new(
                        Uint128::new(1),
                        env.block.clone(),
                        EventData::DcaVaultCreated {},
                    )
                    .build(event_count),
                )
                .unwrap();

            event_count += 1;
        }

        serialised_event_store()
            .save(
                deps.as_mut().storage,
                event_count,
                &to_binary(
                    &EventBuilder::new(
                        Uint128::new(1),
                        env.block.clone(),
                        EventData::DcaVaultCreated {},
                    )
                    .build(event_count),
                )
                .unwrap(),
            )
            .unwrap();

        let migrate_result = migrate_previous_events(deps.as_mut().storage, &mut 50).unwrap_err();

        assert_eq!(
            migrate_result.to_string(),
            "Error: All events have been migrated"
        );
    }

    #[test]
    fn should_migrate_the_previous_events_until_limit_is_reached() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut event_count = 1;

        while event_count <= 100 {
            event_store()
                .save(
                    deps.as_mut().storage,
                    event_count,
                    &EventBuilder::new(
                        Uint128::new(1),
                        env.block.clone(),
                        EventData::DcaVaultCreated {},
                    )
                    .build(event_count),
                )
                .unwrap();

            event_count += 1;
        }

        serialised_event_store()
            .save(
                deps.as_mut().storage,
                event_count,
                &to_binary(
                    &EventBuilder::new(
                        Uint128::new(1),
                        env.block.clone(),
                        EventData::DcaVaultCreated {},
                    )
                    .build(event_count),
                )
                .unwrap(),
            )
            .unwrap();

        let migrate_result = migrate_previous_events(deps.as_mut().storage, &mut 50).unwrap();

        assert_eq!(
            migrate_result.attributes[0],
            ("last_event_migrated_id".to_string(), "50".to_string())
        );
    }

    #[test]
    fn should_migrate_all_events() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut event_count = 0;

        while event_count < 100 {
            event_count += 1;

            event_store()
                .save(
                    deps.as_mut().storage,
                    event_count,
                    &EventBuilder::new(
                        Uint128::new(1),
                        env.block.clone(),
                        EventData::DcaVaultCreated {},
                    )
                    .build(event_count),
                )
                .unwrap();
        }

        serialised_event_store()
            .save(
                deps.as_mut().storage,
                event_count,
                &to_binary(
                    &EventBuilder::new(
                        Uint128::new(1),
                        env.block.clone(),
                        EventData::DcaVaultCreated {},
                    )
                    .build(event_count),
                )
                .unwrap(),
            )
            .unwrap();

        migrate_previous_events(deps.as_mut().storage, &mut 50).unwrap();
        migrate_previous_events(deps.as_mut().storage, &mut 50).unwrap_err();

        assert_eq!(
            event_store()
                .range(deps.as_ref().storage, None, None, Order::Ascending)
                .into_iter()
                .count(),
            serialised_event_store()
                .range(deps.as_ref().storage, None, None, Order::Ascending)
                .into_iter()
                .count(),
        );
    }

    #[test]
    fn should_migrate_the_events_correctly() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let event_to_migrate = EventBuilder::new(
            Uint128::new(1),
            env.block.clone(),
            EventData::DcaVaultCreated {},
        )
        .build(1);

        event_store()
            .save(deps.as_mut().storage, 1, &event_to_migrate)
            .unwrap();

        event_store()
            .save(
                deps.as_mut().storage,
                2,
                &EventBuilder::new(
                    Uint128::new(1),
                    env.block.clone(),
                    EventData::DcaVaultCreated {},
                )
                .build(2),
            )
            .unwrap();

        serialised_event_store()
            .save(
                deps.as_mut().storage,
                2,
                &to_binary(
                    &EventBuilder::new(
                        Uint128::new(1),
                        env.block.clone(),
                        EventData::DcaVaultCreated {},
                    )
                    .build(2),
                )
                .unwrap(),
            )
            .unwrap();

        migrate_previous_events(deps.as_mut().storage, &mut 50).unwrap_err();

        let migrated_event: Event = from_binary(
            &serialised_event_store()
                .load(deps.as_mut().storage, 1)
                .unwrap(),
        )
        .unwrap();

        assert_eq!(migrated_event, event_to_migrate);
    }
}
