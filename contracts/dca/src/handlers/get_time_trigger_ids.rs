use crate::{
    helpers::validation_helpers::assert_page_limit_is_valid, msg::TriggerIdsResponse,
    state::triggers::get_time_triggers,
};
use cosmwasm_std::{Deps, Env, StdResult};

pub fn get_time_trigger_ids(
    deps: Deps,
    env: Env,
    limit: Option<u16>,
) -> StdResult<TriggerIdsResponse> {
    assert_page_limit_is_valid(deps.storage, limit)?;

    Ok(TriggerIdsResponse {
        trigger_ids: get_time_triggers(deps.storage, env.block.time, limit)?,
    })
}
