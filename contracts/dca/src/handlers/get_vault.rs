use crate::{msg::VaultResponse, state::vaults::get_vault as fetch_vault};
use cosmwasm_std::{Deps, StdResult, Uint128};

pub fn get_vault_handler(deps: Deps, vault_id: Uint128) -> StdResult<VaultResponse> {
    let vault = fetch_vault(deps.storage, vault_id)?;

    Ok(VaultResponse { vault })
}
