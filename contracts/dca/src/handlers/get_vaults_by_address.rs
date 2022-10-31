use crate::state::triggers::get_trigger;
use crate::state::vaults::get_vaults_by_address as fetch_vaults_by_address;
use crate::vault::VaultDto;
use crate::{msg::VaultsResponse, validation_helpers::assert_page_limit_is_valid};
use base::vaults::vault::VaultStatus;
use cosmwasm_std::{Addr, Deps, StdResult};

pub fn get_vaults_by_address(
    deps: Deps,
    address: Addr,
    status: Option<VaultStatus>,
    start_after: Option<u128>,
    limit: Option<u16>,
) -> StdResult<VaultsResponse> {
    deps.api.addr_validate(&address.to_string())?;
    assert_page_limit_is_valid(deps.storage, limit)?;

    let vaults = fetch_vaults_by_address(deps.storage, address, status, start_after, limit)?;

    let vault_dtos = vaults
        .into_iter()
        .map(|vault| {
            let trigger = get_trigger(deps.storage, vault.id).ok();
            VaultDto::from_vault(&vault, trigger.map(|t| t.configuration))
        })
        .collect();

    Ok(VaultsResponse { vaults: vault_dtos })
}
