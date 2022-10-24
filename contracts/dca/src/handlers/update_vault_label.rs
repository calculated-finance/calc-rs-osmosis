use crate::{
    error::ContractError, state::vault_store, validation_helpers::assert_sender_is_admin,
    vault::Vault,
};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, Uint128};

pub fn update_vault_label(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
    vault_id: Uint128,
    label: String,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.as_ref(), info.sender)?;

    vault_store().update(
        deps.storage,
        vault_id.into(),
        |existing_vault| -> Result<Vault, ContractError> {
            match existing_vault {
                Some(mut existing_vault) => {
                    existing_vault.label = label.clone();

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

    Ok(Response::default()
        .add_attribute("method", "update_vault_label")
        .add_attribute("label", label))
}
