use crate::error::ContractError;
use crate::state::cache::VAULT_CACHE;
use crate::state::events::create_event;
use crate::state::vaults::get_vault;
use crate::types::event::{EventBuilder, EventData};
use base::helpers::message_helpers::{
    get_attribute_in_event, get_coin_from_display_formatted_coin,
};
use cosmwasm_std::SubMsgResult;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, Reply, Response};

pub fn after_z_delegation(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = VAULT_CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id.into())?;

    match reply.result {
        SubMsgResult::Ok(_) => {
            let z_delegate_response = reply.result.into_result().unwrap();

            let validator_address =
                get_attribute_in_event(&z_delegate_response.events, "delegate", "validator")?;

            let display_formatted_coin =
                get_attribute_in_event(&z_delegate_response.events, "delegate", "amount")?;

            let delegation = get_coin_from_display_formatted_coin(display_formatted_coin);

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block,
                    EventData::DcaVaultZDelegationSucceeded {
                        validator_address,
                        delegation,
                    },
                ),
            )?;
        }
        SubMsgResult::Err(_) => {
            create_event(
                deps.storage,
                EventBuilder::new(vault.id, env.block, EventData::DcaVaultDelegationFailed {}),
            )?;
        }
    }

    Ok(Response::new()
        .add_attribute("method", "after_z_delegation")
        .add_attribute("vault_id", vault.id.to_string()))
}
