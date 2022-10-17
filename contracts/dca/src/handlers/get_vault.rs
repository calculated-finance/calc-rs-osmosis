use crate::{
    msg::VaultResponse,
    state::{trigger_store, vault_store},
};
use base::triggers::trigger::Trigger;
use cosmwasm_std::{Deps, Order, StdResult, Uint128};

pub fn get_vault(deps: Deps, vault_id: Uint128) -> StdResult<VaultResponse> {
    Ok(VaultResponse {
        vault: vault_store().load(deps.storage, vault_id.into())?,
        triggers: trigger_store()
            .idx
            .vault_id
            .sub_prefix(vault_id.into())
            .range(deps.storage, None, None, Order::Ascending)
            .map(|result| result.unwrap().1)
            .collect::<Vec<Trigger>>(),
    })
}
