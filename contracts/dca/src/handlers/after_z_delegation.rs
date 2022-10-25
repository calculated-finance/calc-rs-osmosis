use crate::error::ContractError;
use crate::state::{create_event, get_vault, CACHE};
use base::events::event::{EventBuilder, EventData};
use base::helpers::message_helpers::{
    find_first_attribute_by_key, find_first_event_by_type, get_coin_from_display_formatted_coin,
};
use cosmwasm_std::SubMsgResult;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, Reply, Response};

pub fn after_z_delegation(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id.into())?;

    match reply.result {
        SubMsgResult::Ok(_) => {
            let z_delegate_response = reply.result.into_result().unwrap();
            let z_delegate_event =
                find_first_event_by_type(&z_delegate_response.events, "delegate").unwrap();

            let validator_address =
                find_first_attribute_by_key(&z_delegate_event.attributes, "validator")
                    .unwrap()
                    .value
                    .clone();

            let display_formatted_coin =
                find_first_attribute_by_key(&z_delegate_event.attributes, "amount")
                    .unwrap()
                    .value
                    .clone();

            let delegation_amount = get_coin_from_display_formatted_coin(display_formatted_coin);

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block,
                    EventData::DCAVaultZDelegationSucceeded {
                        validator_address,
                        delegation: delegation_amount,
                    },
                ),
            )?;
        }
        SubMsgResult::Err(_) => {
            create_event(
                deps.storage,
                EventBuilder::new(vault.id, env.block, EventData::DCAVaultDelegationFailed),
            )?;
        }
    }

    Ok(Response::new()
        .add_attribute("method", "after_z_delegation")
        .add_attribute("vault_id", vault.id.to_string()))
}
