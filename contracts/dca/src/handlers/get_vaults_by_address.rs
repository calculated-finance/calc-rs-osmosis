use crate::{msg::VaultsResponse, state::vault_store, vault::Vault};
use cosmwasm_std::{Addr, Deps, Order, StdResult};

pub fn get_vaults_by_address(deps: Deps, address: Addr) -> StdResult<VaultsResponse> {
    deps.api.addr_validate(&address.to_string())?;

    let vaults = vault_store()
        .idx
        .owner
        .sub_prefix(address)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|result| result.unwrap().1)
        .collect::<Vec<Vault>>();

    Ok(VaultsResponse { vaults })
}
