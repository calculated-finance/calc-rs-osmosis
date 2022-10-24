use crate::{
    msg::EventsResponse, state::event_store, validation_helpers::assert_page_limit_is_valid,
};
use base::events::event::Event;
use cosmwasm_std::{Deps, StdResult};
use cw_storage_plus::Bound;

pub fn get_events(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u8>,
) -> StdResult<EventsResponse> {
    assert_page_limit_is_valid(limit)?;

    let events = event_store()
        .range(
            deps.storage,
            start_after.map(Bound::exclusive),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit.unwrap_or(30u8) as usize)
        .map(|result| result.unwrap().1)
        .collect::<Vec<Event>>();

    Ok(EventsResponse { events })
}
