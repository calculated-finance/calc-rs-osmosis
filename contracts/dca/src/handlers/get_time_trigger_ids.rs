use crate::{msg::TriggerIdsResponse, state::TRIGGER_IDS_BY_TARGET_TIME};
use cosmwasm_std::{Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

pub fn get_time_trigger_ids(deps: Deps, env: Env) -> StdResult<TriggerIdsResponse> {
    Ok(TriggerIdsResponse {
        trigger_ids: TRIGGER_IDS_BY_TARGET_TIME
            .range(
                deps.storage,
                None,
                Some(Bound::inclusive(env.block.time.seconds())),
                Order::Ascending,
            )
            .flat_map(|result| result.unwrap().1)
            .map(|trigger_id| trigger_id.into())
            .collect::<Vec<Uint128>>(),
    })
}
