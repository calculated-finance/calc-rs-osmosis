use crate::contract::{
    AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID, AFTER_FIN_SWAP_REPLY_ID,
};
use crate::error::ContractError;
use crate::state::cache::{Cache, LimitOrderCache, CACHE, LIMIT_ORDER_CACHE};
use crate::state::events::create_event;
use crate::state::triggers::{delete_trigger, save_trigger};
use crate::state::vaults::{get_vault, update_vault};
use crate::validation_helpers::assert_target_time_is_in_past;
use base::events::event::{EventBuilder, EventData, ExecutionSkippedReason};
use base::helpers::time_helpers::get_next_target_time;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use base::vaults::vault::{PositionType, VaultStatus};
use cosmwasm_std::StdError;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, Response, Uint128};
use fin_helpers::limit_orders::create_withdraw_limit_order_sub_msg;
use fin_helpers::queries::{query_base_price, query_order_details, query_quote_price};
use fin_helpers::swaps::{create_fin_swap_with_slippage, create_fin_swap_without_slippage};

pub fn execute_trigger_handler(
    deps: DepsMut,
    env: Env,
    trigger_id: Uint128,
) -> Result<Response, ContractError> {
    let response = Response::new().add_attribute("method", "execute_trigger");
    Ok(execute_trigger(deps, env, trigger_id, response)?)
}

pub fn execute_trigger(
    deps: DepsMut,
    env: Env,
    vault_id: Uint128,
    response: Response,
) -> Result<Response, ContractError> {
    let vault = get_vault(deps.storage, vault_id.into())?;

    let position_type = vault.get_position_type();

    if vault.is_scheduled() {
        update_vault(deps.storage, vault.id, |stored_value| match stored_value {
            Some(mut existing_vault) => {
                existing_vault.status = VaultStatus::Active;
                existing_vault.started_at = Some(env.block.time);
                Ok(existing_vault)
            }
            None => Err(StdError::NotFound {
                kind: format!(
                    "vault for address: {} with id: {}",
                    vault.owner.clone(),
                    vault.id
                ),
            }),
        })?;
    }

    let current_fin_price = match position_type {
        PositionType::Enter => query_base_price(deps.querier, vault.pair.address.clone()),
        PositionType::Exit => query_quote_price(deps.querier, vault.pair.address.clone()),
    };

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.to_owned(),
            EventData::DCAVaultExecutionTriggered {
                base_denom: vault.pair.base_denom.clone(),
                quote_denom: vault.pair.quote_denom.clone(),
                asset_price: current_fin_price.clone(),
            },
        ),
    )?;

    match vault
        .trigger
        .clone()
        .expect(format!("trigger for vault id {}", vault.id).as_str())
    {
        TriggerConfiguration::Time { target_time } => {
            assert_target_time_is_in_past(env.block.time, target_time)?;

            if vault.price_threshold_exceeded(current_fin_price) {
                create_event(
                    deps.storage,
                    EventBuilder::new(
                        vault.id,
                        env.block.to_owned(),
                        EventData::DCAVaultExecutionSkipped {
                            reason: ExecutionSkippedReason::PriceThresholdExceeded {
                                price: current_fin_price,
                            },
                        },
                    ),
                )?;

                delete_trigger(deps.storage, vault.id)?;

                save_trigger(
                    deps.storage,
                    Trigger {
                        vault_id: vault.id,
                        configuration: TriggerConfiguration::Time {
                            target_time: get_next_target_time(
                                env.block.time,
                                target_time,
                                vault.time_interval.clone(),
                            ),
                        },
                    },
                )?;

                return Ok(response.to_owned());
            };

            let fin_swap_msg = match vault.slippage_tolerance {
                Some(tolerance) => create_fin_swap_with_slippage(
                    vault.pair.address.clone(),
                    current_fin_price,
                    tolerance,
                    vault.get_swap_amount(),
                    AFTER_FIN_SWAP_REPLY_ID,
                ),
                None => create_fin_swap_without_slippage(
                    vault.pair.address.clone(),
                    vault.get_swap_amount(),
                    AFTER_FIN_SWAP_REPLY_ID,
                ),
            };

            CACHE.save(
                deps.storage,
                &Cache {
                    vault_id: vault.id,
                    owner: vault.owner.clone(),
                },
            )?;

            Ok(response.add_submessage(fin_swap_msg))
        }
        TriggerConfiguration::FINLimitOrder { order_idx, .. } => {
            if let Some(order_idx) = order_idx {
                let (offer_amount, original_offer_amount, filled) =
                    query_order_details(deps.querier, vault.pair.address.clone(), order_idx);

                let limit_order_cache = LimitOrderCache {
                    order_idx,
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
                    vault.pair.address,
                    order_idx,
                    AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID,
                );

                let cache: Cache = Cache {
                    vault_id: vault.id,
                    owner: vault.owner.clone(),
                };

                CACHE.save(deps.storage, &cache)?;

                return Ok(response.add_submessage(fin_withdraw_sub_msg));
            } else {
                return Err(ContractError::CustomError {
                    val: String::from("fin limit order has not been created"),
                });
            }
        }
    }
}
