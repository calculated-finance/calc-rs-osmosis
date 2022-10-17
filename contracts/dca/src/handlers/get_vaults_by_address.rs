use crate::{msg::VaultsResponse, state::vault_store, vault::Vault};
use cosmwasm_std::{Deps, Order, StdResult};

pub fn get_vaults_by_address(deps: Deps, address: String) -> StdResult<VaultsResponse> {
    let validated_address = deps.api.addr_validate(&address)?;

    let vaults = vault_store()
        .idx
        .owner
        .sub_prefix(validated_address.clone())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|result| result.unwrap().1)
        .collect::<Vec<Vault>>();

    Ok(VaultsResponse { vaults })
}
