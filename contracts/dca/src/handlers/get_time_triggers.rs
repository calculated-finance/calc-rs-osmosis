use crate::{msg::TriggersResponse, state::trigger_store};
use base::triggers::trigger::Trigger;
use cosmwasm_std::{Deps, StdResult};

pub fn get_time_triggers(deps: Deps) -> StdResult<TriggersResponse> {
    let all_time_triggers_on_heap: StdResult<Vec<_>> = trigger_store()
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let triggers: Vec<Trigger> = all_time_triggers_on_heap
        .unwrap()
        .iter()
        .map(|t| t.1.clone())
        .collect();

    Ok(TriggersResponse { triggers })
}
