use crate::{
    msg::VaultsResponse, state::vault_store, validation_helpers::assert_page_limit_is_valid,
    vault::Vault,
};
use cosmwasm_std::{Addr, Deps, StdResult};
use cw_storage_plus::Bound;

pub fn get_vaults_by_address(
    deps: Deps,
    address: Addr,
    start_after: Option<u128>,
    limit: Option<u8>,
) -> StdResult<VaultsResponse> {
    deps.api.addr_validate(&address.to_string())?;
    assert_page_limit_is_valid(limit)?;

    let vaults = vault_store()
        .idx
        .owner
        .sub_prefix(address)
        .range(
            deps.storage,
            start_after.map(|vault_id| Bound::exclusive((vault_id, vault_id))),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit.unwrap_or(30u8) as usize)
        .map(|result| result.unwrap().1)
        .collect::<Vec<Vault>>();

    Ok(VaultsResponse { vaults })
}
