use base::executions::dca_execution::DCAExecutionInformation;
use base::executions::execution::{Execution, ExecutionBuilder};
use base::helpers::message_helpers::{find_first_attribute_by_key, find_first_event_by_type};
use base::helpers::time_helpers::{get_next_target_time, target_time_elapsed};
use base::pair::Pair;
use base::triggers::time_trigger::{TimeInterval, TimeTrigger};
use base::triggers::trigger::{Trigger, TriggerBuilder, TriggerVariant};
use base::vaults::dca_vault::{DCAConfiguration, PositionType};
use base::vaults::vault::{Vault, VaultBuilder};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, Decimal256, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdResult, SubMsg, Timestamp, Uint128, Uint64, WasmMsg,
};
use cw2::set_contract_version;
use kujira::fin::{BookResponse, ExecuteMsg as FINExecuteMsg, QueryMsg as FINQueryMsg};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, ExecutionsResponse, InstantiateMsg, MigrateMsg, PairsResponse, QueryMsg,
    TriggersResponse, VaultResponse, VaultsResponse,
};
use crate::validation_helpers::{
    validate_asset_denom, validate_funds, validate_number_of_executions, validate_sender_is_admin,
    validate_sender_is_admin_or_vault_owner, validate_target_start_time,
};

use crate::state::{
    Cache, Config, ACTIVE_VAULTS, CACHE, CANCELLED_VAULTS, CONFIG, EXECUTIONS, FIN_PRICE_TRIGGERS,
    FIN_PRICE_TRIGGERS_BY_ORDER_ID, INACTIVE_VAULTS, PAIRS, TIME_TRIGGERS,
};

const CONTRACT_NAME: &str = "crates.io:calc-dca";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DCA_SWAP_REPLY_ID: u64 = 1;

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config::from(msg.clone());
    config.validate(deps.api)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePair {
            address,
            base_denom,
            quote_denom,
        } => create_pair(deps, env, info, address, base_denom, quote_denom),
        ExecuteMsg::DeletePair { address } => delete_pair(deps, env, info, address),
        ExecuteMsg::CreateVaultWithTimeTrigger {
            pair_address,
            position_type,
            slippage_tolerance,
            swap_amount,
            total_triggers,
            time_interval,
            target_start_time_utc_seconds,
        } => create_vault_with_time_trigger(
            deps,
            env,
            info,
            pair_address,
            position_type,
            slippage_tolerance,
            swap_amount,
            total_triggers,
            time_interval,
            target_start_time_utc_seconds,
        ),
        ExecuteMsg::ExecuteTimeTriggerById { trigger_id } => {
            execute_time_trigger_by_id(deps, env, trigger_id)
        }
        ExecuteMsg::CancelVaultByAddressAndId { address, vault_id } => {
            cancel_vault_by_address_and_id(deps, info, address, vault_id)
        }
    }
}

fn create_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
    base_denom: String,
    quote_denom: String,
) -> Result<Response, ContractError> {
    validate_sender_is_admin(deps.as_ref(), info.sender)?;

    let validated_pair_address: Addr = deps.api.addr_validate(&address)?;

    let pair: Pair = Pair {
        address: validated_pair_address.clone(),
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
    };

    let existing_pair = PAIRS.may_load(deps.storage, validated_pair_address.clone())?;
    match existing_pair {
        Some(_pair) => Err(ContractError::CustomError {
            val: String::from("pair already exists at given address"),
        }),
        None => {
            PAIRS.save(deps.storage, validated_pair_address.clone(), &pair)?;
            Ok(Response::new()
                .add_attribute("method", "create_pair")
                .add_attribute("address", validated_pair_address.to_string())
                .add_attribute("base_denom", base_denom)
                .add_attribute("quote_denom", quote_denom))
        }
    }
}

fn delete_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    validate_sender_is_admin(deps.as_ref(), info.sender)?;

    let validated_pair_address: Addr = deps.api.addr_validate(&address)?;

    PAIRS.remove(deps.storage, validated_pair_address.clone());

    Ok(Response::new()
        .add_attribute("method", "delete_pair")
        .add_attribute("address", validated_pair_address.to_string()))
}

pub fn create_vault_with_time_trigger(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_address: String,
    position_type: PositionType,
    slippage_tolerance: Option<Decimal256>,
    swap_amount: Uint128,
    total_triggers: u16,
    time_interval: TimeInterval,
    target_start_time_utc_seconds: Option<Uint64>,
) -> Result<Response, ContractError> {
    validate_funds(info.funds.clone())?;

    // if no target start time is given execute immediately
    let target_start_time: Timestamp = match target_start_time_utc_seconds {
        Some(time) => Timestamp::from_seconds(time.u64()),
        None => env.block.time,
    };

    validate_target_start_time(env.block.time, target_start_time)?;

    let validated_pair_address = deps.api.addr_validate(&pair_address)?;
    let existing_pair = PAIRS.load(deps.storage, validated_pair_address)?;

    // validate funds denom matches denom of pair
    validate_asset_denom(
        existing_pair.clone(),
        info.funds.clone(),
        position_type.clone(),
    )?;

    // validate all assets will be swapped with none remaining
    validate_number_of_executions(info.funds[0].clone(), swap_amount, total_triggers)?;

    let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        config.vault_count = config.vault_count.checked_add(Uint128::new(1))?;
        config.trigger_count = config.trigger_count.checked_add(Uint128::new(1))?;
        Ok(config)
    })?;

    let trigger = TriggerBuilder::new()
        .id(config.trigger_count)
        .owner(info.sender.clone())
        .vault_id(config.vault_count)
        .time_interval(time_interval)
        .triggers_remaining(total_triggers)
        .target_time(target_start_time)
        .build();

    let vault: Vault<DCAConfiguration> = VaultBuilder::new()
        .id(config.vault_count)
        .owner(info.sender.clone())
        .balance(info.funds[0].clone())
        .pair_address(existing_pair.address)
        .pair_base_denom(existing_pair.base_denom)
        .pair_quote_denom(existing_pair.quote_denom)
        .swap_amount(swap_amount)
        .slippage_tolerance(slippage_tolerance)
        .position_type(position_type)
        .trigger_id(trigger.id)
        .trigger_variant(trigger.variant.clone())
        .build();

    TIME_TRIGGERS.save(deps.storage, trigger.id.u128(), &trigger)?;

    ACTIVE_VAULTS.save(deps.storage, (info.sender, vault.id.u128()), &vault)?;

    EXECUTIONS.save(deps.storage, vault.id.into(), &Vec::new())?;

    Ok(Response::new()
        .add_attribute("method", "create_vault")
        .add_attribute("id", config.vault_count.to_string())
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id))
}

fn cancel_vault_by_address_and_id(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let validated_address = deps.api.addr_validate(&address)?;
    let vault: Vault<DCAConfiguration> =
        ACTIVE_VAULTS.load(deps.storage, (validated_address.clone(), vault_id.into()))?;
    validate_sender_is_admin_or_vault_owner(deps.as_ref(), vault.owner.clone(), info.sender)?;

    match vault.trigger_variant {
        TriggerVariant::Time => TIME_TRIGGERS.remove(deps.storage, vault.trigger_id.into()),
        TriggerVariant::Price => {
            let trigger = FIN_PRICE_TRIGGERS.load(deps.storage, vault.trigger_id.into())?;
            FIN_PRICE_TRIGGERS.remove(deps.storage, trigger.id.into());
            FIN_PRICE_TRIGGERS_BY_ORDER_ID
                .remove(deps.storage, trigger.configuration.order_idx.into());
        }
    };

    let balance = vault.get_current_balance();

    CANCELLED_VAULTS.save(
        deps.storage,
        (validated_address.clone(), vault_id.into()),
        &vault,
    )?;

    ACTIVE_VAULTS.remove(deps.storage, (validated_address, vault_id.into()));

    let bank_message = BankMsg::Send {
        to_address: vault.owner.to_string(),
        amount: vec![balance.clone()],
    };

    Ok(Response::new()
        .add_attribute("method", "cancel_vault_by_address_and_id")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_message(bank_message))
}

fn execute_time_trigger_by_id(
    deps: DepsMut,
    env: Env,
    trigger_id: Uint128,
) -> Result<Response, ContractError> {
    let trigger = TIME_TRIGGERS.load(deps.storage, trigger_id.into())?;

    let vault = ACTIVE_VAULTS.load(
        deps.storage,
        (trigger.owner.clone(), trigger.vault_id.into()),
    )?;

    // move this into validation method
    if !target_time_elapsed(env.block.time, trigger.configuration.target_time) {
        return Err(ContractError::CustomError {
            val: String::from("vault execution time has not yet elapsed"),
        });
    }

    let fin_swap_message = match vault.configuration.slippage_tolerance {
        Some(tolerance) => {
            let book_query_message = FINQueryMsg::Book {
                limit: Some(1),
                offset: None,
            };

            let book_response: BookResponse = deps
                .querier
                .query_wasm_smart(
                    vault.configuration.pair.address.clone(),
                    &book_query_message,
                )
                .unwrap();

            let belief_price = match vault.configuration.position_type {
                PositionType::Enter => book_response.base[0].quote_price,
                PositionType::Exit => book_response.quote[0].quote_price,
            };

            FINExecuteMsg::Swap {
                belief_price: Some(belief_price),
                max_spread: Some(tolerance),
                offer_asset: None,
                to: None,
            }
        }
        None => FINExecuteMsg::Swap {
            belief_price: None,
            max_spread: None,
            offer_asset: None,
            to: None,
        },
    };

    let coin_to_send_with_message = Coin {
        denom: vault.get_swap_denom().clone(),
        amount: if trigger.configuration.triggers_remaining == 1 {
            vault.balances[0].current.amount
        } else {
            vault.configuration.swap_amount
        },
    };

    let execute_message = WasmMsg::Execute {
        contract_addr: vault.configuration.pair.address.into_string(),
        msg: to_binary(&fin_swap_message)?,
        funds: vec![coin_to_send_with_message],
    };

    let sub_message = SubMsg {
        id: DCA_SWAP_REPLY_ID,
        msg: CosmosMsg::Wasm(execute_message),
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Always,
    };

    let cache: Cache = Cache {
        vault_id: vault.id,
        owner: vault.owner,
    };
    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_attribute("method", "execute_time_trigger_by_id")
        .add_submessage(sub_message))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        DCA_SWAP_REPLY_ID => after_dca_swap(deps, env, reply),
        id => Err(ContractError::CustomError {
            val: format!("unknown reply id: {}", id),
        }),
    }
}

pub fn after_dca_swap(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = ACTIVE_VAULTS.load(deps.storage, (cache.owner.clone(), cache.vault_id.into()))?;
    let trigger: Trigger<TimeTrigger> =
        TIME_TRIGGERS.load(deps.storage, vault.trigger_id.into())?;
    let executions: Vec<Execution<DCAExecutionInformation>> =
        EXECUTIONS.load(deps.storage, vault.id.into())?;
    let number_of_previous_executions: u16 = executions.len().try_into().unwrap();

    let mut attributes: Vec<Attribute> = Vec::new();
    let mut messages: Vec<CosmosMsg> = Vec::new();

    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_swap_response = reply.result.into_result().unwrap();

            let wasm_trade_event =
                find_first_event_by_type(&fin_swap_response.events, String::from("wasm-trade"))
                    .unwrap();

            let base_amount = find_first_attribute_by_key(
                &wasm_trade_event.attributes,
                String::from("base_amount"),
            )
            .unwrap()
            .value
            .clone();

            let quote_amount = find_first_attribute_by_key(
                &wasm_trade_event.attributes,
                String::from("quote_amount"),
            )
            .unwrap()
            .value
            .clone();

            let coin_sent_with_swap: Coin = match vault.configuration.position_type {
                PositionType::Enter => {
                    let parsed_quote_amount = quote_amount.parse::<u128>().unwrap();
                    Coin {
                        denom: vault.configuration.pair.quote_denom.clone(),
                        amount: Uint128::from(parsed_quote_amount),
                    }
                }
                PositionType::Exit => {
                    let parsed_base_amount = base_amount.parse::<u128>().unwrap();
                    Coin {
                        denom: vault.configuration.pair.base_denom.clone(),
                        amount: Uint128::from(parsed_base_amount),
                    }
                }
            };

            let coin_received_from_swap: Coin = match vault.configuration.position_type.clone() {
                PositionType::Enter => {
                    let parsed_base_amount = base_amount.parse::<u128>().unwrap();
                    Coin {
                        denom: vault.configuration.pair.base_denom,
                        amount: Uint128::from(parsed_base_amount),
                    }
                }
                PositionType::Exit => {
                    let parsed_quote_amount = quote_amount.parse::<u128>().unwrap();
                    Coin {
                        denom: vault.configuration.pair.quote_denom,
                        amount: Uint128::from(parsed_quote_amount),
                    }
                }
            };

            // if asset kuji and staking enabled stake instead
            let bank_message_to_vault_owner: BankMsg = BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![coin_received_from_swap.clone()],
            };

            messages.push(CosmosMsg::Bank(bank_message_to_vault_owner));

            let updated_vault = ACTIVE_VAULTS.update(
                deps.storage,
                (vault.owner.clone(), vault.id.into()),
                |vault| -> Result<Vault<DCAConfiguration>, ContractError> {
                    match vault {
                        Some(mut existing_vault) => {
                            existing_vault.balances[0].current.amount -= coin_sent_with_swap.amount;
                            Ok(existing_vault)
                        }
                        None => Err(ContractError::CustomError {
                            val: format!(
                                "could not find vault for address: {} with id: {}",
                                cache.owner, cache.vault_id
                            ),
                        }),
                    }
                },
            )?;

            if trigger.configuration.is_final_trigger() {
                // move these things things into a function
                INACTIVE_VAULTS.save(
                    deps.storage,
                    (vault.owner.clone(), vault.id.into()),
                    &updated_vault,
                )?;

                ACTIVE_VAULTS.remove(deps.storage, (vault.owner.clone(), vault.id.into()));
                TIME_TRIGGERS.remove(deps.storage, vault.trigger_id.u128());
            } else {
                let next_trigger_time = get_next_target_time(
                    env.block.time,
                    trigger.configuration.target_time,
                    trigger.configuration.time_interval,
                );

                TIME_TRIGGERS.update(deps.storage, trigger.id.into(), |existing_trigger| {
                    match existing_trigger {
                        Some(mut trigger) => {
                            trigger.configuration.target_time = next_trigger_time;
                            trigger.configuration.triggers_remaining -= 1;
                            Ok(trigger)
                        }
                        None => Err(ContractError::CustomError {
                            val: format!("could not trigger with id: {}", trigger.id),
                        }),
                    }
                })?;
            }

            let execution = ExecutionBuilder::new()
                .vault_id(vault.id)
                .sequence_id(number_of_previous_executions + 1)
                .block_height(env.block.height)
                .success(coin_sent_with_swap.clone(), coin_received_from_swap.clone())
                .build();

            EXECUTIONS.update(deps.storage, vault.id.into(), |existing_executions: Option<Vec<Execution<DCAExecutionInformation>>>| -> Result<Vec<Execution<DCAExecutionInformation>>, ContractError> {
                match existing_executions {
                    Some(mut executions) => {
                        executions.push(execution);
                        Ok(executions)
                    },
                    None => {
                        Err(
                            ContractError::CustomError {
                                val: format!(
                                    "could not find execution history for vault with id: {}", 
                                    cache.vault_id
                                )
                            }
                        )
                    }
                }
            })?;

            attributes.push(Attribute::new("status", "success"));
            attributes.push(Attribute::new(
                "coin_sent_with_swap",
                coin_sent_with_swap.to_string(),
            ));
            attributes.push(Attribute::new(
                "coin_received_from_swap",
                coin_received_from_swap.to_string(),
            ));
        }
        cosmwasm_std::SubMsgResult::Err(_) => {
            // move into trigger
            let next_trigger_time = get_next_target_time(
                env.block.time,
                trigger.configuration.target_time,
                trigger.configuration.time_interval,
            );

            TIME_TRIGGERS.update(deps.storage, trigger.id.into(), |existing_trigger| {
                match existing_trigger {
                    Some(mut trigger) => {
                        trigger.configuration.target_time = next_trigger_time;
                        Ok(trigger)
                    }
                    None => Err(ContractError::CustomError {
                        val: format!("could not trigger with id: {}", trigger.id),
                    }),
                }
            })?;

            let execution = ExecutionBuilder::new()
                .vault_id(vault.id)
                .sequence_id(number_of_previous_executions + 1)
                .block_height(env.block.height)
                .fail_slippage()
                .build();

            EXECUTIONS.update(deps.storage, vault.id.into(), |existing_executions: Option<Vec<Execution<DCAExecutionInformation>>>| -> Result<Vec<Execution<DCAExecutionInformation>>, ContractError> {
                match existing_executions {
                    Some(mut executions) => {
                        executions.push(execution);
                        Ok(executions)
                    },
                    None => {
                        Err(
                            ContractError::CustomError {
                                val: format!(
                                    "could not find execution history for vault with id: {}",
                                    cache.vault_id
                                )
                            }
                        )
                    }
                }
            })?;

            attributes.push(Attribute::new("status", "slippage"));
        }
    };

    CACHE.remove(deps.storage);

    Ok(Response::default()
        .add_attribute("method", "after_execute_vault_by_address_and_id")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_attributes(attributes)
        .add_messages(messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAllPairs {} => to_binary(&get_all_pairs(deps)?),
        QueryMsg::GetAllTimeTriggers {} => to_binary(&get_all_time_triggers(deps)?),
        QueryMsg::GetAllActiveVaults {} => to_binary(&get_all_active_vaults(deps)?),
        QueryMsg::GetActiveVaultByAddressAndId { address, vault_id } => to_binary(
            &get_active_vault_by_address_and_id(deps, address, vault_id)?,
        ),
        QueryMsg::GetAllActiveVaultsByAddress { address } => {
            to_binary(&get_all_active_vaults_by_address(deps, address)?)
        }
        QueryMsg::GetInactiveVaultByAddressAndId { address, vault_id } => to_binary(
            &get_inactive_vault_by_address_and_id(deps, address, vault_id)?,
        ),
        QueryMsg::GetAllInactiveVaultsByAddress { address } => {
            to_binary(&get_all_inactive_vaults_by_address(deps, address)?)
        }
        QueryMsg::GetAllExecutionsByVaultId { vault_id } => {
            to_binary(&get_all_executions_by_vault_id(deps, vault_id)?)
        }
    }
}

fn get_all_pairs(deps: Deps) -> StdResult<PairsResponse> {
    let all_pairs_on_heap: StdResult<Vec<_>> = PAIRS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let pairs: Vec<Pair> = all_pairs_on_heap
        .unwrap()
        .iter()
        .map(|p| p.1.clone())
        .collect();

    Ok(PairsResponse { pairs })
}

fn get_all_time_triggers(deps: Deps) -> StdResult<TriggersResponse<TimeTrigger>> {
    let all_time_triggers_on_heap: StdResult<Vec<_>> = TIME_TRIGGERS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let triggers: Vec<Trigger<TimeTrigger>> = all_time_triggers_on_heap
        .unwrap()
        .iter()
        .map(|t| t.1.clone())
        .collect();

    Ok(TriggersResponse { triggers })
}

fn get_active_vault_by_address_and_id(
    deps: Deps,
    address: String,
    vault_id: Uint128,
) -> StdResult<VaultResponse> {
    let validated_address = deps.api.addr_validate(&address)?;
    let vault = ACTIVE_VAULTS.load(deps.storage, (validated_address, vault_id.into()))?;
    Ok(VaultResponse { vault })
}

fn get_all_active_vaults(deps: Deps) -> StdResult<VaultsResponse> {
    let all_active_vaults_on_heap: StdResult<Vec<_>> = ACTIVE_VAULTS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let vaults: Vec<Vault<DCAConfiguration>> = all_active_vaults_on_heap
        .unwrap()
        .iter()
        .map(|v| v.1.clone())
        .collect();

    Ok(VaultsResponse { vaults })
}

fn get_all_active_vaults_by_address(deps: Deps, address: String) -> StdResult<VaultsResponse> {
    let validated_address = deps.api.addr_validate(&address)?;

    let active_vaults_on_heap: StdResult<Vec<_>> = ACTIVE_VAULTS
        .prefix(validated_address)
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let vaults: Vec<Vault<DCAConfiguration>> = active_vaults_on_heap
        .unwrap()
        .iter()
        .map(|v| -> Vault<DCAConfiguration> { v.1.clone() })
        .collect();

    Ok(VaultsResponse { vaults })
}

fn get_inactive_vault_by_address_and_id(
    deps: Deps,
    address: String,
    vault_id: Uint128,
) -> StdResult<VaultResponse> {
    let validated_address = deps.api.addr_validate(&address)?;
    let vault = INACTIVE_VAULTS.load(deps.storage, (validated_address, vault_id.into()))?;
    Ok(VaultResponse { vault })
}

fn get_all_inactive_vaults_by_address(deps: Deps, address: String) -> StdResult<VaultsResponse> {
    let validated_address = deps.api.addr_validate(&address)?;

    let all_inactive_vaults_on_heap: StdResult<Vec<_>> = INACTIVE_VAULTS
        .prefix(validated_address)
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let vaults: Vec<Vault<DCAConfiguration>> = all_inactive_vaults_on_heap
        .unwrap()
        .iter()
        .map(|v| -> Vault<DCAConfiguration> { v.1.clone() })
        .collect();

    Ok(VaultsResponse { vaults })
}

fn get_all_executions_by_vault_id(deps: Deps, vault_id: Uint128) -> StdResult<ExecutionsResponse> {
    let all_executions_on_heap: Vec<Execution<DCAExecutionInformation>> =
        EXECUTIONS.load(deps.storage, vault_id.into())?;

    Ok(ExecutionsResponse {
        executions: all_executions_on_heap,
    })
}
