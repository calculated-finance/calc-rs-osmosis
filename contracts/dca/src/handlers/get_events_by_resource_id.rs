use crate::{msg::EventsResponse, state::event_store};
use base::events::event::Event;
use cosmwasm_std::{Deps, Order, StdResult, Uint128};

pub fn get_events_by_resource_id(deps: Deps, resource_id: Uint128) -> StdResult<EventsResponse> {
    let events = event_store()
        .idx
        .resource_id
        .sub_prefix(resource_id.into())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|result| result.unwrap().1)
        .collect::<Vec<Event>>();

    Ok(EventsResponse { events })
}
