use crate::msg::TriggerIdResponse;
use crate::state::TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX;
#[cfg(not(feature = "library"))]
use cosmwasm_std::Uint128;
use cosmwasm_std::{Deps, StdResult};

pub fn get_trigger_id_by_fin_limit_order_idx(
    deps: Deps,
    order_idx: Uint128,
) -> StdResult<TriggerIdResponse> {
    let trigger_id = TRIGGER_ID_BY_FIN_LIMIT_ORDER_IDX
        .load(deps.storage, order_idx.into())?
        .into();

    Ok(TriggerIdResponse { trigger_id })
}
