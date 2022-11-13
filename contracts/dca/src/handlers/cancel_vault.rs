use crate::contract::AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID;
use crate::error::ContractError;
use crate::state::cache::{Cache, LimitOrderCache, CACHE, LIMIT_ORDER_CACHE};
use crate::state::events::create_event;
use crate::state::triggers::delete_trigger;
use crate::state::vaults::{get_vault, update_vault};
use crate::types::vault::Vault;
use crate::validation_helpers::{
    assert_sender_is_admin_or_vault_owner, assert_vault_is_not_cancelled,
};
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::TriggerConfiguration;
use base::vaults::vault::VaultStatus;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, DepsMut, Response, Uint128};
use cosmwasm_std::{Coin, CosmosMsg, Env, MessageInfo, StdError, StdResult};
use fin_helpers::limit_orders::create_retract_order_sub_msg;
use fin_helpers::queries::query_order_details;

pub fn cancel_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let vault = get_vault(deps.storage, vault_id)?;

    assert_sender_is_admin_or_vault_owner(deps.storage, vault.owner.clone(), info.sender.clone())?;
    assert_vault_is_not_cancelled(&vault)?;

    create_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block, EventData::DcaVaultCancelled {}),
    )?;

    match vault.trigger {
        Some(TriggerConfiguration::Time { .. }) => {
            delete_trigger(deps.storage, vault.id.into())?;
            refund_vault_balance(deps, vault)
        }
        Some(TriggerConfiguration::FinLimitOrder { order_idx, .. }) => {
            cancel_fin_limit_order_trigger(deps, order_idx.unwrap(), vault)
        }
        None => refund_vault_balance(deps, vault),
    }
}

fn refund_vault_balance(deps: DepsMut, vault: Vault) -> Result<Response, ContractError> {
    let mut response = Response::new()
        .add_attribute("method", "cancel_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id);

    if vault.balance.amount.gt(&Uint128::zero()) {
        response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: vault.owner.to_string(),
            amount: vec![vault.balance.clone()],
        }))
    }

    update_vault(
        deps.storage,
        vault.id.into(),
        |existing_vault| -> StdResult<Vault> {
            match existing_vault {
                Some(mut existing_vault) => {
                    existing_vault.status = VaultStatus::Cancelled;
                    existing_vault.balance = Coin::new(0, existing_vault.get_swap_denom());
                    Ok(existing_vault)
                }
                None => Err(StdError::NotFound {
                    kind: format!("vault for address: {} with id: {}", vault.owner, vault.id),
                }),
            }
        },
    )?;

    Ok(response)
}

fn cancel_fin_limit_order_trigger(
    deps: DepsMut,
    order_idx: Uint128,
    vault: Vault,
) -> Result<Response, ContractError> {
    let (offer_amount, original_offer_amount, filled) =
        query_order_details(deps.querier, vault.pair.address.clone(), order_idx);

    let limit_order_cache = LimitOrderCache {
        order_idx,
        offer_amount,
        original_offer_amount,
        filled,
    };

    LIMIT_ORDER_CACHE.save(deps.storage, &limit_order_cache)?;

    let fin_retract_order_sub_msg = create_retract_order_sub_msg(
        vault.pair.address,
        order_idx,
        AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID,
    );

    let cache = Cache {
        vault_id: vault.id,
        owner: vault.owner.clone(),
    };

    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_attribute("method", "cancel_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_submessage(fin_retract_order_sub_msg))
}
