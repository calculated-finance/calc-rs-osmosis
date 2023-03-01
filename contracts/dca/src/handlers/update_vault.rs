use crate::{
    error::ContractError,
    helpers::validation_helpers::{assert_vault_is_not_cancelled, asset_sender_is_vault_owner},
    state::vaults::{get_vault, update_vault},
};
use cosmwasm_std::{DepsMut, MessageInfo, Response, Uint128};

pub fn update_vault_handler(
    deps: DepsMut,
    info: MessageInfo,
    vault_id: Uint128,
    label: Option<String>,
) -> Result<Response, ContractError> {
    let mut vault = get_vault(deps.storage, vault_id)?;

    assert_vault_is_not_cancelled(&vault)?;

    vault.label = label;
    update_vault(deps.storage, &vault)?;

    asset_sender_is_vault_owner(vault.owner, info.sender)?;

    Ok(Response::default().add_attribute("method", "update_vault"))
}
