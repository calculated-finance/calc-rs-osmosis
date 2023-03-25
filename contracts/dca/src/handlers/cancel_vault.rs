use crate::error::ContractError;
use crate::helpers::validation_helpers::{
    assert_sender_is_admin_or_vault_owner, assert_vault_is_not_cancelled,
};
use crate::state::disburse_escrow_tasks::save_disburse_escrow_task;
use crate::state::events::create_event;
use crate::state::triggers::delete_trigger;
use crate::state::vaults::{get_vault, update_vault};
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::TriggerConfiguration;
use base::vaults::vault::VaultStatus;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, DepsMut, Response, Uint128};
use cosmwasm_std::{Coin, CosmosMsg, Env, MessageInfo};
use fin_helpers::limit_orders::{create_retract_order_msg, create_withdraw_limit_order_msg};
use fin_helpers::queries::query_order_details;

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

    if let Some(trigger) = vault.trigger {
        match trigger {
            TriggerConfiguration::FinLimitOrder { order_idx, .. } => {
                if let Some(order_idx) = order_idx {
                    let limit_order =
                        query_order_details(deps.querier, vault.pair.address.clone(), order_idx)
                            .expect(&format!(
                                "Fin limit order exists at pair {}",
                                vault.pair.address.clone()
                            ));

                    if limit_order.offer_amount > Uint128::zero() {
                        messages.push(create_retract_order_msg(
                            vault.pair.address.clone(),
                            order_idx,
                        ));
                    }

                    if limit_order.filled_amount > Uint128::zero() {
                        messages.push(create_withdraw_limit_order_msg(
                            vault.pair.address.clone(),
                            order_idx,
                        ));
                    }
                }
            }
            _ => {}
        }

        delete_trigger(deps.storage, vault.id)?;
    }

    Ok(Response::new()
        .add_attribute("method", "cancel_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_messages(messages))
}
