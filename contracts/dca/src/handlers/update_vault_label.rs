use crate::{
    error::ContractError, state::vault_store, validation_helpers::asset_sender_is_vault_owner,
    vault::Vault,
};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, Uint128};

pub fn update_vault(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
    vault_id: Uint128,
    label: Option<String>,
) -> Result<Response, ContractError> {
    let updated_vault = vault_store().update(
        deps.storage,
        vault_id.into(),
        |existing_vault| -> Result<Vault, ContractError> {
            match existing_vault {
                Some(mut existing_vault) => {
                    if let Some(label) = label {
                        existing_vault.label = label;
                    }
                    Ok(existing_vault)
                }
                None => Err(ContractError::CustomError {
                    val: format!(
                        "could not find vault for address: {} with id: {}",
                        address.clone(),
                        vault_id
                    ),
                }),
            }
        },
    )?;

    asset_sender_is_vault_owner(updated_vault.owner, info.sender)?;

    Ok(Response::default().add_attribute("method", "update_vault"))
}
