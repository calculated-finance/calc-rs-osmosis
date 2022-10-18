use crate::{
    msg::VaultResponse,
    state::{trigger_store, vault_store},
};
use base::triggers::trigger::Trigger;
use cosmwasm_std::{Deps, Order, StdResult, Uint128};

pub fn get_vault(deps: Deps, address: String, vault_id: Uint128) -> StdResult<VaultResponse> {
    let vault = vault_store().load(deps.storage, vault_id.into())?;

    if vault.owner != address {
        return Err(cosmwasm_std::StdError::NotFound {
            kind: "vault".to_string(),
        });
    }

    Ok(VaultResponse {
        vault,
        triggers: trigger_store()
            .idx
            .vault_id
            .sub_prefix(vault_id.into())
            .range(deps.storage, None, None, Order::Ascending)
            .map(|result| result.unwrap().1)
            .collect::<Vec<Trigger>>(),
    })
}
