use crate::{msg::VaultResponse, state::vault_store};
use cosmwasm_std::{Deps, StdResult, Uint128};

pub fn get_vault_by_id(deps: Deps, vault_id: Uint128) -> StdResult<VaultResponse> {
    let vault = vault_store().load(deps.storage, vault_id.into())?;
    Ok(VaultResponse { vault })
}
