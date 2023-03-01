use crate::helpers::validation_helpers::assert_page_limit_is_valid;
use crate::msg::DataFixesResponse;
use crate::state::data_fixes::{data_fix_store, DataFix};
use cosmwasm_std::{from_binary, Deps, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

pub fn get_data_fixes_by_resource_id(
    deps: Deps,
    resource_id: Uint128,
    start_after: Option<u64>,
    limit: Option<u16>,
) -> StdResult<DataFixesResponse> {
    assert_page_limit_is_valid(deps.storage, limit)?;

    let fixes = data_fix_store()
        .idx
        .resource_id
        .prefix(resource_id.into())
        .range(
            deps.storage,
            start_after.map(Bound::exclusive),
            None,
            Order::Ascending,
        )
        .take(limit.unwrap_or(30) as usize)
        .map(|result| from_binary(&result.unwrap().1).expect("Deserialised data fix"))
        .collect::<Vec<DataFix>>();

    Ok(DataFixesResponse { fixes })
}
