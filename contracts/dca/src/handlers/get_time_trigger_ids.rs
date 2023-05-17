use crate::{
    helpers::validation::assert_page_limit_is_valid, msg::TriggerIdsResponse,
    state::triggers::get_time_triggers,
};
use cosmwasm_std::{Deps, Env, StdResult};

pub fn get_time_trigger_ids_handler(
    deps: Deps,
    env: Env,
    limit: Option<u16>,
) -> StdResult<TriggerIdsResponse> {
    assert_page_limit_is_valid(limit)?;

    Ok(TriggerIdsResponse {
        trigger_ids: get_time_triggers(deps.storage, env.block.time, limit)?,
    })
}
