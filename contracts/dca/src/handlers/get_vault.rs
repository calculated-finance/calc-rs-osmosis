use crate::{
    msg::VaultResponse, state::triggers::get_trigger, state::vaults::get_vault as fetch_vault,
};
use cosmwasm_std::{Addr, Deps, StdError, StdResult, Uint128};

pub fn get_vault(deps: Deps, address: Addr, vault_id: Uint128) -> StdResult<VaultResponse> {
    let vault = fetch_vault(deps.storage, vault_id.into())?;

    if vault.owner != address {
        return Err(StdError::NotFound {
            kind: format!("vault for address: {} with id: {}", address, vault.id),
        });
    }

    let trigger = get_trigger(deps.storage, vault.id).ok();

    Ok(VaultResponse { vault, trigger })
}
