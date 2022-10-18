use crate::{msg::TriggerIdsResponse, state::TRIGGER_IDS_BY_TARGET_TIME};
use cosmwasm_std::{Deps, Order, StdResult, Uint128, Uint64};
use cw_storage_plus::Bound;

pub fn get_time_trigger_ids(
    deps: Deps,
    before_target_time_in_utc_seconds: Uint64,
) -> StdResult<TriggerIdsResponse> {
    Ok(TriggerIdsResponse {
        trigger_ids: TRIGGER_IDS_BY_TARGET_TIME
            .range(
                deps.storage,
                None,
                Some(Bound::inclusive(before_target_time_in_utc_seconds)),
                Order::Ascending,
            )
            .flat_map(|result| result.unwrap().1)
            .map(|trigger_id| trigger_id.into())
            .collect::<Vec<Uint128>>(),
    })
}
