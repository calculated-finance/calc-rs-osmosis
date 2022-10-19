use crate::error::ContractError;
use crate::state::{create_event, vault_store, CACHE};
use base::events::event::{EventBuilder, EventData};
use cosmwasm_std::SubMsgResult;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, Reply, Response};

pub fn delegation_succeeded(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = vault_store().load(deps.storage, cache.vault_id.into())?;

    match reply.result {
        SubMsgResult::Ok(_) => {
            create_event(
                deps.storage,
                EventBuilder::new(vault.id, env.block, EventData::DCAVaultDelegationSucceeded),
            )?;
        }
        SubMsgResult::Err(_) => {
            create_event(
                deps.storage,
                EventBuilder::new(vault.id, env.block, EventData::DCAVaultDelegationFailed),
            )?;
        }
    }

    Ok(Response::new().add_attribute("method", "delegation_succeeded"))
}
