use crate::{
    msg::VaultResponse,
    state::{get_trigger, vault_store},
};
use cosmwasm_std::{Deps, StdResult, Uint128};

pub fn get_vault(deps: Deps, address: String, vault_id: Uint128) -> StdResult<VaultResponse> {
    let vault = vault_store().load(deps.storage, vault_id.into())?;

    if vault.owner != address {
        return Err(cosmwasm_std::StdError::NotFound {
            kind: "vault".to_string(),
        });
    }

    let trigger = get_trigger(deps.storage, vault.id)?;

    Ok(VaultResponse { vault, trigger })
}
