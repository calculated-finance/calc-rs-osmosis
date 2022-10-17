use crate::contract::FIN_LIMIT_ORDER_RETRACTED_ID;
use crate::error::ContractError;
use crate::state::{
    create_event, trigger_store, vault_store, Cache, LimitOrderCache, CACHE, LIMIT_ORDER_CACHE,
};
use crate::validation_helpers::assert_sender_is_admin_or_vault_owner;
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::TriggerConfiguration;
use cosmwasm_std::Env;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, DepsMut, Response, Uint128};
use fin_helpers::limit_orders::create_retract_order_sub_msg;
use fin_helpers::queries::query_order_details;

pub fn cancel_vault(
    deps: DepsMut,
    env: Env,
    address: String,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let validated_address = deps.api.addr_validate(&address)?;

    let vault = vault_store().load(deps.storage, vault_id.into())?;
    assert_sender_is_admin_or_vault_owner(deps.as_ref(), vault.owner.clone(), validated_address)?;

    let trigger = trigger_store().load(deps.storage, vault_id.into())?;

    create_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block, EventData::DCAVaultCancelled),
    )?;

    match trigger.configuration {
        TriggerConfiguration::Time { .. } => cancel_time_trigger(deps, vault),
        TriggerConfiguration::FINLimitOrder { order_idx, .. } => {
            cancel_fin_limit_order_trigger(deps, trigger.id, order_idx.unwrap(), vault)
        }
    }
}

fn cancel_time_trigger(deps: DepsMut, vault: Vault) -> Result<Response, ContractError> {
    trigger_store().remove(deps.storage, vault.id.into())?;
    vault_store().remove(deps.storage, vault.id.into())?;

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
    trigger_id: Uint128,
    order_idx: Uint128,
    vault: Vault,
) -> Result<Response, ContractError> {
    let (offer_amount, original_offer_amount, filled) =
        query_order_details(deps.querier, vault.pair.address.clone(), order_idx);

    let limit_order_cache = LimitOrderCache {
        trigger_id,
        offer_amount,
        original_offer_amount,
        filled,
    };

    LIMIT_ORDER_CACHE.save(deps.storage, &limit_order_cache)?;

    let fin_retract_order_sub_msg =
        create_retract_order_sub_msg(vault.pair.address, order_idx, FIN_LIMIT_ORDER_RETRACTED_ID);

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
