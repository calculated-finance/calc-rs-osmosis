use crate::contract::{FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_ID, FIN_SWAP_COMPLETED_ID};
use crate::error::ContractError;
use crate::state::{
    create_event, trigger_store, vault_store, Cache, LimitOrderCache, TimeTriggerCache, CACHE,
    LIMIT_ORDER_CACHE, TIME_TRIGGER_CACHE,
};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData};
use base::helpers::time_helpers::target_time_elapsed;
use base::pair::Pair;
use base::triggers::trigger::TriggerConfiguration;
use base::vaults::vault::{PositionType, VaultStatus};
use cosmwasm_std::Order;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, Response, Timestamp, Uint128};
use fin_helpers::limit_orders::create_withdraw_limit_order_sub_msg;
use fin_helpers::queries::{query_base_price, query_order_details, query_quote_price};
use fin_helpers::swaps::{create_fin_swap_with_slippage, create_fin_swap_without_slippage};

pub fn execute_trigger(
    deps: DepsMut,
    env: Env,
    trigger_id: Uint128,
) -> Result<Response, ContractError> {
    // TODO: refactor into multiple trigger stores?
    let trigger = match trigger_store().may_load(deps.storage, trigger_id.into())? {
        Some(trigger) => trigger,
        None => trigger_store()
            .idx
            .order_idx
            .prefix(trigger_id.into())
            .range(deps.storage, None, None, Order::Descending)
            .map(|item| item.map(|(_, trigger)| trigger))
            .last()
            .unwrap()?,
    };

    let vault = vault_store().load(deps.storage, trigger.vault_id.into())?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.to_owned(),
            EventData::DCAVaultExecutionTriggered,
        ),
    )?;

    match trigger.configuration {
        TriggerConfiguration::Time { target_time } => {
            execute_time_trigger(deps, env, vault, trigger_id, target_time)
        }
        TriggerConfiguration::FINLimitOrder { order_idx, .. } => execute_fin_limit_order_trigger(
            deps,
            vault.to_owned(),
            vault.pair.to_owned(),
            trigger_id,
            order_idx.unwrap(),
        ),
    }
}

fn execute_time_trigger(
    deps: DepsMut,
    env: Env,
    vault: Vault,
    trigger_id: Uint128,
    target_time: Timestamp,
) -> Result<Response, ContractError> {
    if !target_time_elapsed(env.block.time, target_time) {
        return Err(ContractError::CustomError {
            val: String::from("trigger execution time has not yet elapsed"),
        });
    }

    if vault.low_funds() {
        vault_store().update(
            deps.storage,
            vault.id.into(),
            |existing_vault| -> Result<Vault, ContractError> {
                match existing_vault {
                    Some(mut existing_vault) => {
                        existing_vault.status = VaultStatus::Inactive;
                        Ok(existing_vault)
                    }
                    None => Err(ContractError::CustomError {
                        val: format!(
                            "could not find vault for address: {} with id: {}",
                            vault.owner.clone(),
                            vault.id
                        ),
                    }),
                }
            },
        )?;
    }

    let fin_swap_msg = match vault.slippage_tolerance {
        Some(tolerance) => {
            let belief_price = match vault.position_type {
                PositionType::Enter => query_base_price(deps.querier, vault.pair.address.clone()),
                PositionType::Exit => query_quote_price(deps.querier, vault.pair.address.clone()),
            };

            create_fin_swap_with_slippage(
                vault.pair.address.clone(),
                belief_price,
                tolerance,
                vault.get_swap_amount(),
                FIN_SWAP_COMPLETED_ID,
            )
        }
        None => create_fin_swap_without_slippage(
            vault.pair.address.clone(),
            vault.get_swap_amount(),
            FIN_SWAP_COMPLETED_ID,
        ),
    };

    TIME_TRIGGER_CACHE.save(deps.storage, &TimeTriggerCache { trigger_id })?;

    CACHE.save(
        deps.storage,
        &Cache {
            vault_id: vault.id,
            owner: vault.owner.clone(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "execute_time_trigger")
        .add_submessage(fin_swap_msg))
}

fn execute_fin_limit_order_trigger(
    deps: DepsMut,
    vault: Vault,
    pair: Pair,
    trigger_id: Uint128,
    order_idx: Uint128,
) -> Result<Response, ContractError> {
    let (offer_amount, original_offer_amount, filled) =
        query_order_details(deps.querier, pair.address.clone(), order_idx);

    let limit_order_cache = LimitOrderCache {
        trigger_id,
        offer_amount,
        original_offer_amount,
        filled,
    };

    LIMIT_ORDER_CACHE.save(deps.storage, &limit_order_cache)?;

    if offer_amount != Uint128::zero() {
        return Err(ContractError::CustomError {
            val: String::from("fin limit order has not been completely filled"),
        });
    }

    let fin_withdraw_sub_msg = create_withdraw_limit_order_sub_msg(
        pair.address,
        order_idx,
        FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_ID,
    );

    let cache: Cache = Cache {
        vault_id: vault.id,
        owner: vault.owner.clone(),
    };

    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_attribute("method", "execute_fin_limit_order_trigger")
        .add_submessage(fin_withdraw_sub_msg))
}
