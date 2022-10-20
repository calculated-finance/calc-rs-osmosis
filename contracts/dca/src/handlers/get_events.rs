use crate::{msg::EventsResponse, state::event_store};
use base::events::event::Event;
use cosmwasm_std::{Deps, StdError, StdResult};
use cw_storage_plus::Bound;

pub fn get_events(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u8>,
) -> StdResult<EventsResponse> {
    if limit.is_some() && limit.unwrap() > 30u8 {
        return Err(StdError::generic_err("limit cannot be greater than 30."));
    }

    let events = event_store()
        .range(
            deps.storage,
            start_after.map(|s| Bound::exclusive(s)),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit.unwrap_or(30u8) as usize)
        .map(|result| result.unwrap().1)
        .collect::<Vec<Event>>();

    Ok(EventsResponse { events })
}
