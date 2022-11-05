use crate::{
    error::ContractError, state::vaults::update_vault, types::vault::Vault,
    validation_helpers::asset_sender_is_vault_owner,
};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, StdError, StdResult, Uint128};

pub fn update_vault_label(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
    vault_id: Uint128,
    label: Option<String>,
) -> Result<Response, ContractError> {
    let updated_vault = update_vault(
        deps.storage,
        vault_id.into(),
        |existing_vault| -> StdResult<Vault> {
            match existing_vault {
                Some(mut existing_vault) => {
                    if let Some(label) = label {
                        existing_vault.label = Some(label);
                    }
                    Ok(existing_vault)
                }
                None => Err(StdError::NotFound {
                    kind: format!("vault for address: {} with id: {}", address, vault_id),
                }),
            }
        },
    )?;

    asset_sender_is_vault_owner(updated_vault.owner, info.sender)?;

    Ok(Response::default().add_attribute("method", "update_vault"))
}
