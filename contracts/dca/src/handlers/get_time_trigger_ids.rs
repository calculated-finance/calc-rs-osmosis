use crate::{
    helpers::validation_helpers::assert_page_limit_is_valid, msg::TriggerIdsResponse,
    state::triggers::TRIGGER_IDS_BY_TARGET_TIME,
};
use cosmwasm_std::{Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

pub fn get_time_trigger_ids(
    deps: Deps,
    env: Env,
    limit: Option<u16>,
) -> StdResult<TriggerIdsResponse> {
    assert_page_limit_is_valid(deps.storage, limit)?;

    Ok(TriggerIdsResponse {
        trigger_ids: TRIGGER_IDS_BY_TARGET_TIME
            .range(
                deps.storage,
                None,
                Some(Bound::inclusive(env.block.time.seconds())),
                Order::Ascending,
            )
            .take(limit.unwrap_or(30) as usize)
            .flat_map(|result| result.unwrap().1)
            .map(|trigger_id| trigger_id.into())
            .collect::<Vec<Uint128>>(),
    })
}
