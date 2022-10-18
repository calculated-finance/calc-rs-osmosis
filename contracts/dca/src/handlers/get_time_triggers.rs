use crate::{msg::TriggersResponse, state::trigger_store};
use base::triggers::trigger::Trigger;
use cosmwasm_std::{Deps, Order, StdResult};

pub fn get_time_triggers(deps: Deps) -> StdResult<TriggersResponse> {
    Ok(TriggersResponse {
        triggers: trigger_store()
            .idx
            .variant
            .prefix(0)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|t| t.unwrap().1.clone())
            .collect::<Vec<Trigger>>(),
    })
}
