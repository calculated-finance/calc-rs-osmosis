use crate::contract::AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID;
use crate::error::ContractError;
use crate::state::{
    create_event, delete_trigger, get_trigger, update_vault, Cache, LimitOrderCache, CACHE,
    LIMIT_ORDER_CACHE,
};
use crate::validation_helpers::assert_sender_is_admin_or_vault_owner;
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::TriggerConfiguration;
use base::vaults::vault::VaultStatus;
use cosmwasm_std::{Addr, Env, StdError, StdResult};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, DepsMut, Response, Uint128};
use fin_helpers::limit_orders::create_retract_order_sub_msg;
use fin_helpers::queries::query_order_details;

pub fn cancel_vault(
    deps: DepsMut,
    env: Env,
    address: Addr,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&address.to_string())?;

    let updated_vault = update_vault(
        deps.storage,
        vault_id.into(),
        |existing_vault| -> StdResult<Vault> {
            match existing_vault {
                Some(mut existing_vault) => {
                    existing_vault.status = VaultStatus::Cancelled;
                    Ok(existing_vault)
                }
                None => Err(StdError::NotFound {
                    kind: format!("vault for address: {} with id: {}", address, vault_id),
                }),
            }
        },
    )?;

    assert_sender_is_admin_or_vault_owner(deps.as_ref(), updated_vault.owner.clone(), address)?;

    create_event(
        deps.storage,
        EventBuilder::new(updated_vault.id, env.block, EventData::DCAVaultCancelled),
    )?;

    let trigger = get_trigger(deps.storage, vault_id.into())?;

    match trigger.configuration {
        TriggerConfiguration::Time { .. } => cancel_time_trigger(deps, updated_vault),
        TriggerConfiguration::FINLimitOrder { order_idx, .. } => {
            cancel_fin_limit_order_trigger(deps, order_idx.unwrap(), updated_vault)
        }
    }
}

fn cancel_time_trigger(deps: DepsMut, vault: Vault) -> Result<Response, ContractError> {
    delete_trigger(deps.storage, vault.id.into())?;

    let refund_bank_msg = BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![vault.balance.clone()],
    };

    Ok(Response::new()
        .add_attribute("method", "cancel_vault")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_message(refund_bank_msg))
}

fn cancel_fin_limit_order_trigger(
    deps: DepsMut,
    order_idx: Uint128,
    vault: Vault,
) -> Result<Response, ContractError> {
    let (offer_amount, original_offer_amount, filled) =
        query_order_details(deps.querier, vault.pair.address.clone(), order_idx);

    let limit_order_cache = LimitOrderCache {
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
