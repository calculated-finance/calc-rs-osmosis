use crate::error::ContractError;
use crate::helpers::validation_helpers::{
    assert_sender_is_admin_or_vault_owner, assert_vault_is_not_cancelled,
};
use crate::state::disburse_escrow_tasks::save_disburse_escrow_task;
use crate::state::events::create_event;
use crate::state::triggers::delete_trigger;
use crate::state::vaults::{get_vault, update_vault};
use crate::types::event::{EventBuilder, EventData};
use crate::types::vault::VaultStatus;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, DepsMut, Response, Uint128};
use cosmwasm_std::{Coin, CosmosMsg, Env, MessageInfo};

pub fn cancel_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let mut vault = get_vault(deps.storage, vault_id)?;

    assert_sender_is_admin_or_vault_owner(deps.storage, vault.owner.clone(), info.sender.clone())?;
    assert_vault_is_not_cancelled(&vault)?;

    create_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block.clone(), EventData::DcaVaultCancelled {}),
    )?;

    if let Some(_) = vault.dca_plus_config {
        save_disburse_escrow_task(
            deps.storage,
            vault.id,
            vault.get_expected_execution_completed_date(env.block.time),
        )?;
    };

    let mut messages: Vec<CosmosMsg> = Vec::new();

    if vault.balance.amount > Uint128::zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: vault.owner.to_string(),
            amount: vec![vault.balance.clone()],
        }));
    }

    vault.status = VaultStatus::Cancelled;
    vault.balance = Coin::new(0, vault.get_swap_denom());

    update_vault(deps.storage, &vault)?;

    delete_trigger(deps.storage, vault.id)?;

    Ok(Response::new()
        .add_attribute("method", "cancel_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_messages(messages))
}
