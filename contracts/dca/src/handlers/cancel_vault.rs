use crate::contract::FIN_LIMIT_ORDER_RETRACTED_ID;
use crate::dca_configuration::DCAConfiguration;
use crate::error::ContractError;
use crate::state::{
    save_event, trigger_store, vault_store, Cache, LimitOrderCache, CACHE, LIMIT_ORDER_CACHE,
    TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID,
};
use crate::validation_helpers::assert_sender_is_admin_or_vault_owner;
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::TriggerConfiguration;
use base::vaults::vault::Vault;
use cosmwasm_std::Env;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, DepsMut, Response, Uint128};
use fin_helpers::limit_orders::create_retract_order_sub_msg;
use fin_helpers::queries::query_order_details;

pub fn cancel_vault_by_address_and_id(
    deps: DepsMut,
    env: Env,
    address: String,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let validated_address = deps.api.addr_validate(&address)?;
    let vault = vault_store().load(deps.storage, vault_id.into())?;

    assert_sender_is_admin_or_vault_owner(deps.as_ref(), vault.owner.clone(), validated_address)?;

    let trigger_store = trigger_store();
    let trigger = trigger_store.load(deps.storage, vault.trigger_id.unwrap().u128())?;

    match trigger.configuration {
        TriggerConfiguration::Time {
            time_interval: _,
            target_time: _,
        } => cancel_vault_with_time_trigger(deps, env, vault),
        TriggerConfiguration::FINLimitOrder {
            target_price: _,
            order_idx: _,
        } => cancel_dca_vault_with_fin_limit_order_trigger(deps, vault),
    }
}

fn cancel_vault_with_time_trigger(
    deps: DepsMut,
    env: Env,
    vault: Vault<DCAConfiguration>,
) -> Result<Response, ContractError> {
    trigger_store().remove(deps.storage, vault.trigger_id.unwrap().into())?;
    vault_store().remove(deps.storage, vault.id.into())?;

    save_event(
        deps.storage,
        EventBuilder::new(vault.id, env.block, EventData::DCAVaultCancelled),
    )?;

    let balance = vault.configuration.balance.clone();

    let refund_bank_msg = BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![balance.clone()],
    };

    Ok(Response::new()
        .add_attribute("method", "cancel_vault_by_address_and_id")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_message(refund_bank_msg))
}

fn cancel_dca_vault_with_fin_limit_order_trigger(
    deps: DepsMut,
    vault: Vault<DCAConfiguration>,
) -> Result<Response, ContractError> {
    TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID.remove(deps.storage, vault.id.u128());

    let fin_limit_order_trigger =
        trigger_store().load(deps.storage, vault.trigger_id.unwrap().u128())?;

    let (offer_amount, original_offer_amount, filled) = query_order_details(
        deps.querier,
        vault.configuration.pair.address.clone(),
        fin_limit_order_trigger.id,
    );

    let limit_order_cache = LimitOrderCache {
        offer_amount,
        original_offer_amount,
        filled,
    };

    LIMIT_ORDER_CACHE.save(deps.storage, &limit_order_cache)?;

    let fin_retract_order_sub_msg = create_retract_order_sub_msg(
        vault.configuration.pair.address,
        fin_limit_order_trigger.id,
        FIN_LIMIT_ORDER_RETRACTED_ID,
    );

    let cache = Cache {
        vault_id: vault.id,
        owner: vault.owner,
    };

    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_attribute("method", "cancel_vault_by_address_and_id")
        .add_submessage(fin_retract_order_sub_msg))
}
