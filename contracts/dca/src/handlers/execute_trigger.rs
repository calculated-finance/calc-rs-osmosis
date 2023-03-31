use crate::contract::AFTER_FIN_SWAP_REPLY_ID;
use crate::error::ContractError;
use crate::helpers::fee_helpers::{get_delegation_fee_rate, get_swap_fee_rate};
use crate::helpers::validation_helpers::{
    assert_contract_is_not_paused, assert_target_time_is_in_past,
};
use crate::helpers::vault_helpers::{get_swap_amount, price_threshold_exceeded};
use crate::msg::ExecuteMsg;
use crate::state::cache::{Cache, SwapCache, CACHE, SWAP_CACHE};
use crate::state::events::create_event;
use crate::state::triggers::{delete_trigger, save_trigger};
use crate::state::vaults::{get_vault, update_vault};
use base::events::event::{EventBuilder, EventData, ExecutionSkippedReason};
use base::helpers::coin_helpers::add_to_coin;
use base::helpers::time_helpers::get_next_target_time;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use base::vaults::vault::VaultStatus;
use cosmwasm_std::{to_binary, Coin, CosmosMsg, Decimal, ReplyOn, StdResult, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Env, Response, Uint128};
use osmosis_helpers::constants::OSMOSIS_SWAP_FEE_RATE;
use osmosis_helpers::queries::{calculate_slippage, query_belief_price, query_price};
use osmosis_helpers::swaps::create_osmosis_swap_message;
use std::cmp::min;
use std::str::FromStr;

pub fn execute_trigger_handler(
    deps: DepsMut,
    env: Env,
    trigger_id: Uint128,
) -> Result<Response, ContractError> {
    assert_contract_is_not_paused(deps.storage)?;
    let response = Response::new().add_attribute("method", "execute_trigger");
    Ok(execute_trigger(deps, env, trigger_id, response)?)
}

pub fn execute_trigger(
    deps: DepsMut,
    env: Env,
    vault_id: Uint128,
    mut response: Response,
) -> Result<Response, ContractError> {
    let mut vault = get_vault(deps.storage, vault_id.into())?;

    delete_trigger(deps.storage, vault.id)?;

    if vault.is_cancelled() {
        return Err(ContractError::CustomError {
            val: format!(
                "vault with id {} is cancelled, and is not available for execution",
                vault.id
            ),
        });
    }

    if vault.trigger.is_none() {
        return Err(ContractError::CustomError {
            val: format!(
                "vault with id {} has no trigger attached, and is not available for execution",
                vault.id
            ),
        });
    }

    match vault
        .trigger
        .clone()
        .expect(format!("trigger for vault id {}", vault.id).as_str())
    {
        TriggerConfiguration::Time { target_time } => {
            assert_target_time_is_in_past(env.block.time, target_time)?;
        }
        TriggerConfiguration::FinLimitOrder { order_idx: _, .. } => {
            unimplemented!()
        }
    }

    if vault.is_scheduled() {
        vault.status = VaultStatus::Active;
        vault.started_at = Some(env.block.time);
    }

    update_vault(deps.storage, &vault)?;

    let belief_price =
        query_belief_price(deps.querier, &env, &vault.pool, &vault.get_swap_denom())?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.to_owned(),
            EventData::DcaVaultExecutionTriggered {
                base_denom: vault.pool.base_denom.clone(),
                quote_denom: vault.pool.quote_denom.clone(),
                asset_price: belief_price.clone(),
            },
        ),
    )?;

    let standard_dca_still_active = vault.dca_plus_config.clone().map_or(
        Ok(false),
        |mut dca_plus_config| -> StdResult<bool> {
            let swap_amount = min(
                dca_plus_config.clone().standard_dca_balance().amount,
                vault.swap_amount,
            );

            if swap_amount.is_zero() {
                return Ok(false);
            }

            let actual_price_result = query_price(
                deps.querier,
                &env,
                &vault.pool,
                &Coin::new(swap_amount.into(), vault.get_swap_denom()),
            );

            if actual_price_result.is_err() {
                let error = actual_price_result.unwrap_err();

                create_event(
                    deps.storage,
                    EventBuilder::new(
                        vault.id,
                        env.block.clone(),
                        EventData::SimulatedDcaVaultExecutionSkipped {
                            reason: if error.to_string().contains("Not enough liquidity to swap") {
                                ExecutionSkippedReason::SlippageToleranceExceeded
                            } else {
                                ExecutionSkippedReason::UnknownFailure
                            },
                        },
                    ),
                )?;

                return Ok(dca_plus_config.has_sufficient_funds());
            }

            let actual_price = actual_price_result.unwrap();

            if let Some(slippage_tolerance) = vault.slippage_tolerance {
                let slippage = calculate_slippage(actual_price, belief_price);

                if slippage > slippage_tolerance {
                    create_event(
                        deps.storage,
                        EventBuilder::new(
                            vault.id,
                            env.block.clone(),
                            EventData::SimulatedDcaVaultExecutionSkipped {
                                reason: ExecutionSkippedReason::SlippageToleranceExceeded,
                            },
                        ),
                    )?;

                    return Ok(dca_plus_config.has_sufficient_funds());
                }
            }

            let fee_rate = get_swap_fee_rate(&deps, &vault)?
                + get_delegation_fee_rate(&deps, &vault)?
                + Decimal::from_str(OSMOSIS_SWAP_FEE_RATE)?;

            let receive_amount = swap_amount * (Decimal::one() / actual_price);

            let fee_amount = receive_amount * fee_rate;

            dca_plus_config.standard_dca_swapped_amount =
                add_to_coin(dca_plus_config.standard_dca_swapped_amount, swap_amount);

            dca_plus_config.standard_dca_received_amount =
                add_to_coin(dca_plus_config.standard_dca_received_amount, receive_amount);

            vault.dca_plus_config = Some(dca_plus_config.clone());

            update_vault(deps.storage, &vault)?;

            create_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block.clone(),
                    EventData::SimulatedDcaVaultExecutionCompleted {
                        sent: Coin::new(swap_amount.into(), vault.get_swap_denom()),
                        received: Coin::new(receive_amount.into(), vault.get_receive_denom()),
                        fee: Coin::new(fee_amount.into(), vault.get_receive_denom()),
                    },
                ),
            )?;

            Ok(dca_plus_config.has_sufficient_funds())
        },
    )?;

    if vault.is_active() || standard_dca_still_active {
        save_trigger(
            deps.storage,
            Trigger {
                vault_id: vault.id,
                configuration: TriggerConfiguration::Time {
                    target_time: get_next_target_time(
                        env.block.time,
                        match vault.trigger {
                            Some(TriggerConfiguration::Time { target_time }) => target_time,
                            _ => env.block.time,
                        },
                        vault.time_interval.clone(),
                    ),
                },
            },
        )?;
    } else {
        if vault.is_dca_plus() {
            response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::DisburseEscrow { vault_id: vault.id })?,
                funds: vec![],
            }));
        }

        return Ok(response);
    }

    if price_threshold_exceeded(&deps.as_ref(), &env, &vault, belief_price)? {
        create_event(
            deps.storage,
            EventBuilder::new(
                vault.id,
                env.block.to_owned(),
                EventData::DcaVaultExecutionSkipped {
                    reason: ExecutionSkippedReason::PriceThresholdExceeded {
                        price: belief_price,
                    },
                },
            ),
        )?;

        return Ok(response.to_owned());
    };

    CACHE.save(
        deps.storage,
        &Cache {
            vault_id: vault.id,
            owner: vault.owner.clone(),
        },
    )?;

    SWAP_CACHE.save(
        deps.storage,
        &SwapCache {
            swap_denom_balance: deps
                .querier
                .query_balance(&env.contract.address, &vault.get_swap_denom())?,
            receive_denom_balance: deps
                .querier
                .query_balance(&env.contract.address, &vault.get_receive_denom())?,
        },
    )?;

    response = response.add_attribute(
        "calc_swap_fee_rate",
        get_swap_fee_rate(&deps, &vault)?.to_string(),
    );
    response = response.add_attribute(
        "delegation_fee_rate",
        get_delegation_fee_rate(&deps, &vault)?.to_string(),
    );
    response = response.add_attribute(
        "osmisis_swap_fee_rate",
        Decimal::from_str(OSMOSIS_SWAP_FEE_RATE)
            .unwrap()
            .to_string(),
    );

    Ok(response.add_submessage(create_osmosis_swap_message(
        deps.querier,
        &env,
        vault.pool.clone(),
        get_swap_amount(&deps.as_ref(), &env, vault.clone())?,
        vault.slippage_tolerance,
        Some(AFTER_FIN_SWAP_REPLY_ID),
        Some(ReplyOn::Always),
    )?))
}
