use crate::state::events::event_store;
use crate::{msg::EventsResponse, validation_helpers::assert_page_limit_is_valid};
use base::events::event::Event;
use cosmwasm_std::{from_binary, Deps, StdResult};
use cw_storage_plus::Bound;

pub fn get_events(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u16>,
) -> StdResult<EventsResponse> {
    assert_page_limit_is_valid(deps.storage, limit)?;

    let events = event_store()
        .range(
            deps.storage,
            start_after.map(Bound::exclusive),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit.unwrap_or(30) as usize)
        .map(|result| from_binary::<Event>(&result.unwrap().1).expect("deserialised event"))
        .collect::<Vec<Event>>();

    Ok(EventsResponse { events })
}
