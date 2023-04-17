use crate::state::vaults::get_vaults_by_address as fetch_vaults_by_address;
use crate::types::vault::VaultStatus;
use crate::{helpers::validation_helpers::assert_page_limit_is_valid, msg::VaultsResponse};
use cosmwasm_std::{Addr, Deps, StdResult};

pub fn get_vaults_by_address(
    deps: Deps,
    address: Addr,
    status: Option<VaultStatus>,
    start_after: Option<u128>,
    limit: Option<u16>,
) -> StdResult<VaultsResponse> {
    deps.api.addr_validate(address.as_ref())?;
    assert_page_limit_is_valid(deps.storage, limit)?;

    let vaults = fetch_vaults_by_address(deps.storage, address, status, start_after, limit)?;

    Ok(VaultsResponse { vaults })
}
