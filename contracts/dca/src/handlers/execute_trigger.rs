use crate::contract::{FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_ID, FIN_SWAP_COMPLETED_ID};
use crate::dca_configuration::DCAConfiguration;
use crate::error::ContractError;
use crate::state::{
    save_event, trigger_store, vault_store, Cache, LimitOrderCache, CACHE, LIMIT_ORDER_CACHE,
};
use base::events::event::{EventBuilder, EventData};
use base::helpers::time_helpers::target_time_elapsed;
use base::pair::Pair;
use base::triggers::trigger::TriggerConfiguration;
use base::vaults::vault::{PositionType, Vault, VaultStatus};
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
    let trigger = trigger_store().load(deps.storage, trigger_id.into())?;
    let vault = vault_store().load(deps.storage, trigger.vault_id.into())?;

    save_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.to_owned(),
            EventData::VaultExecutionTriggered {
                trigger_id: trigger.id,
            },
        ),
    )?;

    match trigger.configuration {
        TriggerConfiguration::Time {
            time_interval: _,
            target_time,
        } => execute_time_trigger(deps, env, vault.to_owned(), target_time),
        TriggerConfiguration::FINLimitOrder {
            target_price: _,
            order_idx,
        } => execute_fin_limit_order_trigger(
            deps,
            vault.to_owned(),
            vault.configuration.pair.to_owned(),
            order_idx.unwrap(),
        ),
    }
}

fn execute_time_trigger(
    deps: DepsMut,
    env: Env,
    vault: Vault<DCAConfiguration>,
    target_time: Timestamp,
) -> Result<Response, ContractError> {
    if !target_time_elapsed(env.block.time, target_time) {
        return Err(ContractError::CustomError {
            val: String::from("trigger execution time has not yet elapsed"),
        });
    }

    // change the status of the vault so frontend knows
    if vault.configuration.low_funds() {
        vault_store().update(
            deps.storage,
            vault.id.into(),
            |existing_vault| -> Result<Vault<DCAConfiguration>, ContractError> {
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

    let fin_swap_msg = match vault.configuration.slippage_tolerance {
        Some(tolerance) => {
            let belief_price = match vault.configuration.position_type {
                PositionType::Enter => {
                    query_base_price(deps.querier, vault.configuration.pair.address.clone())
                }
                PositionType::Exit => {
                    query_quote_price(deps.querier, vault.configuration.pair.address.clone())
                }
            };

            create_fin_swap_with_slippage(
                vault.configuration.pair.address.clone(),
                belief_price,
                tolerance,
                vault.configuration.get_swap_amount(),
                FIN_SWAP_COMPLETED_ID,
            )
        }
        None => create_fin_swap_without_slippage(
            vault.configuration.pair.address.clone(),
            vault.configuration.get_swap_amount(),
            FIN_SWAP_COMPLETED_ID,
        ),
    };

    let cache: Cache = Cache {
        vault_id: vault.id,
        owner: vault.owner.clone(),
    };
    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_attribute("method", "execute_time_trigger_by_id")
        .add_submessage(fin_swap_msg))
}

fn execute_fin_limit_order_trigger(
    deps: DepsMut,
    vault: Vault<DCAConfiguration>,
    pair: Pair,
    order_idx: Uint128,
) -> Result<Response, ContractError> {
    let (offer_amount, original_offer_amount, filled) =
        query_order_details(deps.querier, pair.address.clone(), order_idx);

    let limit_order_cache = LimitOrderCache {
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
        .add_attribute("method", "execute_fin_limit_order_trigger_by_order_idx")
        .add_submessage(fin_withdraw_sub_msg))
}
