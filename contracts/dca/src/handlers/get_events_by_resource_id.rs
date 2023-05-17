use crate::state::config::get_config;
use crate::state::events::event_store;
use crate::types::event::Event;
use crate::{helpers::validation::assert_page_limit_is_valid, msg::EventsResponse};
use cosmwasm_std::{from_binary, Deps, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

pub fn get_events_by_resource_id_handler(
    deps: Deps,
    resource_id: Uint128,
    start_after: Option<u64>,
    limit: Option<u16>,
    reverse: Option<bool>,
) -> StdResult<EventsResponse> {
    assert_page_limit_is_valid(limit)?;

    let events = event_store()
        .idx
        .resource_id
        .prefix(resource_id.into())
        .range(
            deps.storage,
            reverse.map_or(start_after.map(Bound::exclusive), |_| None),
            reverse.and_then(|_| start_after.map(Bound::exclusive)),
            reverse.map_or(Order::Ascending, |reverse| match reverse {
                true => Order::Descending,
                false => Order::Ascending,
            }),
        )
        .take(
            limit.unwrap_or_else(|| get_config(deps.storage).unwrap().default_page_limit) as usize,
        )
        .flat_map(|result| result.map(|(_, data)| from_binary(&data)))
        .flatten()
        .collect::<Vec<Event>>();

    Ok(EventsResponse { events })
}

#[cfg(test)]
mod get_events_by_resource_id_tests {
    use super::*;
    use crate::{
        state::events::create_events,
        tests::{helpers::instantiate_contract, mocks::ADMIN},
        types::event::{EventBuilder, EventData},
    };
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn events_are_empty() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let events =
            get_events_by_resource_id_handler(deps.as_ref(), Uint128::one(), None, None, None)
                .unwrap()
                .events;

        assert_eq!(events.len(), 0);
    }

    #[test]
    fn events_are_returned() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(
            deps.as_mut().storage,
            vec![
                EventBuilder::default(),
                EventBuilder::default(),
                EventBuilder::default(),
                EventBuilder::new(Uint128::new(2), mock_env().block, EventData::default()),
                EventBuilder::new(Uint128::new(2), mock_env().block, EventData::default()),
            ],
        )
        .unwrap();

        let events =
            get_events_by_resource_id_handler(deps.as_ref(), Uint128::one(), None, None, None)
                .unwrap()
                .events;

        assert_eq!(events.len(), 3);
    }

    #[test]
    fn events_are_not_reversed_when_reverse_is_none() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(
            deps.as_mut().storage,
            vec![
                EventBuilder::default(),
                EventBuilder::default(),
                EventBuilder::default(),
            ],
        )
        .unwrap();

        let events =
            get_events_by_resource_id_handler(deps.as_ref(), Uint128::one(), None, None, None)
                .unwrap()
                .events;

        assert_eq!(events.first().unwrap().id, 1);
        assert_eq!(events.last().unwrap().id, 3);
    }

    #[test]
    fn events_are_not_reversed_when_reverse_is_false() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(
            deps.as_mut().storage,
            vec![
                EventBuilder::default(),
                EventBuilder::default(),
                EventBuilder::default(),
            ],
        )
        .unwrap();

        let events = get_events_by_resource_id_handler(
            deps.as_ref(),
            Uint128::one(),
            None,
            None,
            Some(false),
        )
        .unwrap()
        .events;

        assert_eq!(events.first().unwrap().id, 1);
        assert_eq!(events.last().unwrap().id, 3);
    }

    #[test]
    fn events_are_limited() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(deps.as_mut().storage, vec![EventBuilder::default(); 40]).unwrap();

        let events =
            get_events_by_resource_id_handler(deps.as_ref(), Uint128::one(), None, Some(30), None)
                .unwrap()
                .events;

        assert_eq!(events.len(), 30);
    }

    #[test]
    fn events_are_skipped() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(
            deps.as_mut().storage,
            vec![
                EventBuilder::default(),
                EventBuilder::default(),
                EventBuilder::default(),
            ],
        )
        .unwrap();

        let events =
            get_events_by_resource_id_handler(deps.as_ref(), Uint128::one(), Some(2), None, None)
                .unwrap()
                .events;

        assert_eq!(events.len(), 1);
    }

    #[test]
    fn events_are_reversed() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(
            deps.as_mut().storage,
            vec![
                EventBuilder::default(),
                EventBuilder::default(),
                EventBuilder::default(),
            ],
        )
        .unwrap();

        let events = get_events_by_resource_id_handler(
            deps.as_ref(),
            Uint128::one(),
            None,
            None,
            Some(true),
        )
        .unwrap()
        .events;

        assert_eq!(events.first().unwrap().id, 3);
        assert_eq!(events.last().unwrap().id, 1);
    }

    #[test]
    fn events_are_skipped_and_limited() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(deps.as_mut().storage, vec![EventBuilder::default(); 40]).unwrap();

        let events = get_events_by_resource_id_handler(
            deps.as_ref(),
            Uint128::one(),
            Some(1),
            Some(30),
            None,
        )
        .unwrap()
        .events;

        assert_eq!(events.len(), 30);
        assert_eq!(events.first().unwrap().id, 2);
    }

    #[test]
    fn events_are_skipped_and_reversed() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(
            deps.as_mut().storage,
            vec![
                EventBuilder::default(),
                EventBuilder::default(),
                EventBuilder::default(),
            ],
        )
        .unwrap();

        let events = get_events_by_resource_id_handler(
            deps.as_ref(),
            Uint128::one(),
            Some(3),
            None,
            Some(true),
        )
        .unwrap()
        .events;

        assert_eq!(events.first().unwrap().id, 2);
        assert_eq!(events.last().unwrap().id, 1);
    }

    #[test]
    fn events_are_skipped_reversed_and_limited() {
        let mut deps = mock_dependencies();
        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        create_events(deps.as_mut().storage, vec![EventBuilder::default(); 40]).unwrap();

        let events = get_events_by_resource_id_handler(
            deps.as_ref(),
            Uint128::one(),
            Some(36),
            Some(30),
            Some(true),
        )
        .unwrap()
        .events;

        assert_eq!(events.len(), 30);
        assert_eq!(events.first().unwrap().id, 35);
    }
}
