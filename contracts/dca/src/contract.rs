// vaults balances do not actually go to zero after the last swap
// create vaults that can be deposited into
// vaults that can be topped up
// vaults that can be cancelled
// keep track of balance and swap amount
// always try and swap
// if the swap fails update vault status
// if the vault amount is less than the swap amount then change vault status
// keep track of vault creation time so inactive vaults can be automatically cleaned up infuture
// could look at indexing triggers just by vault id

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, ExecutionsResponse, InstantiateMsg, MigrateMsg, PairsResponse, QueryMsg,
    TriggersResponse, VaultResponse, VaultsResponse,
};
use crate::state::{
    Cache, Config, LimitOrderCache, CACHE, CONFIG, EXECUTIONS,
    FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID, FIN_LIMIT_ORDER_TRIGGERS,
    FIN_LIMIT_ORDER_TRIGGER_IDS_BY_ORDER_IDX, LIMIT_ORDER_CACHE, PAIRS, TIME_TRIGGERS,
    TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID, VAULTS,
};
use crate::validation_helpers::{
    validate_asset_denom_matches_pair_denom, validate_funds, validate_sender_is_admin,
    validate_sender_is_admin_or_vault_owner, validate_swap_amount, validate_target_start_time,
};
use base::executions::dca_execution::DCAExecutionInformation;
use base::executions::execution::{Execution, ExecutionBuilder};
use base::helpers::message_helpers::{find_first_attribute_by_key, find_first_event_by_type};
use base::helpers::time_helpers::{get_next_target_time, target_time_elapsed};
use base::pair::Pair;
use base::triggers::fin_limit_order_configuration::FINLimitOrderConfiguration;
use base::triggers::time_configuration::{TimeConfiguration, TimeInterval};
use base::triggers::trigger::{Trigger, TriggerBuilder, TriggerVariant};
use base::vaults::dca_vault::{DCAConfiguration, DCAStatus, PositionType};
use base::vaults::vault::{Vault, VaultBuilder};
use cosmwasm_std::Decimal256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_binary, Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdResult, Timestamp, Uint128, Uint64,
};
use cw2::set_contract_version;
use fin_helpers::codes::{ERROR_SWAP_INSUFFICIENT_FUNDS, ERROR_SWAP_SLIPPAGE};
use fin_helpers::limit_orders::{
    create_limit_order_sub_msg, create_retract_order_sub_msg, create_withdraw_limit_order_sub_msg,
};
use fin_helpers::queries::{query_base_price, query_order_details, query_quote_price};
use fin_helpers::swaps::{create_fin_swap_with_slippage, create_fin_swap_without_slippage};

const CONTRACT_NAME: &str = "crates.io:calc-dca";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const SWAP_REPLY_ID: u64 = 1;
const SUBMIT_ORDER_REPLY_ID: u64 = 2;
const EXECUTE_TRIGGER_WITHDRAW_ORDER_REPLY_ID: u64 = 3;
const RETRACT_ORDER_REPLY_ID: u64 = 4;
const CANCEL_TRIGGER_WITHDRAW_ORDER_REPLY_ID: u64 = 5;

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let config: Config = _msg.into();
    config.validate(_deps.api)?;
    CONFIG.save(_deps.storage, &config)?;
    Ok(Response::default())
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
            time_interval,
            target_start_time_utc_seconds,
        ),
        ExecuteMsg::CreateVaultWithFINLimitOrderTrigger {
            pair_address,
            position_type,
            slippage_tolerance,
            swap_amount,
            time_interval,
            target_price,
        } => create_vault_with_fin_limit_order_trigger(
            deps,
            env,
            info,
            pair_address,
            position_type,
            slippage_tolerance,
            swap_amount,
            time_interval,
            target_price,
        ),
        ExecuteMsg::ExecuteTimeTriggerById { trigger_id } => {
            execute_time_trigger_by_id(deps, env, trigger_id)
        }
        ExecuteMsg::ExecuteFINLimitOrderTriggerByOrderIdx { order_idx } => {
            execute_fin_limit_order_trigger_by_order_idx(deps, env, order_idx)
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

fn create_vault_with_time_trigger(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_address: String,
    position_type: PositionType,
    slippage_tolerance: Option<Decimal256>,
    swap_amount: Uint128,
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

    validate_asset_denom_matches_pair_denom(
        existing_pair.clone(),
        info.funds.clone(),
        position_type.clone(),
    )?;

    validate_swap_amount(swap_amount, info.funds[0].clone())?;

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
        .target_time(target_start_time)
        .build();

    let vault: Vault<DCAConfiguration, DCAStatus> = VaultBuilder::new()
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

    VAULTS.save(deps.storage, (info.sender, vault.id.u128()), &vault)?;

    EXECUTIONS.save(deps.storage, vault.id.into(), &Vec::new())?;

    Ok(Response::new()
        .add_attribute("method", "create_vault_with_time_trigger")
        .add_attribute("id", config.vault_count.to_string())
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id))
}

fn create_vault_with_fin_limit_order_trigger(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pair_address: String,
    position_type: PositionType,
    slippage_tolerance: Option<Decimal256>,
    swap_amount: Uint128,
    time_interval: TimeInterval,
    target_price: Decimal256,
) -> Result<Response, ContractError> {
    validate_funds(info.funds.clone())?;

    let validated_pair_address = deps.api.addr_validate(&pair_address)?;
    let existing_pair = PAIRS.load(deps.storage, validated_pair_address)?;

    validate_asset_denom_matches_pair_denom(
        existing_pair.clone(),
        info.funds.clone(),
        position_type.clone(),
    )?;

    validate_swap_amount(swap_amount, info.funds[0].clone())?;

    let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
        config.vault_count = config.vault_count.checked_add(Uint128::new(1))?;
        Ok(config)
    })?;

    // trigger information is updated upon successful limit order creation
    let vault: Vault<DCAConfiguration, DCAStatus> = VaultBuilder::new()
        .id(config.vault_count)
        .owner(info.sender.clone())
        .balance(info.funds[0].clone())
        .pair_address(existing_pair.address.clone())
        .pair_base_denom(existing_pair.base_denom)
        .pair_quote_denom(existing_pair.quote_denom)
        .swap_amount(swap_amount)
        .slippage_tolerance(slippage_tolerance)
        .position_type(position_type)
        .build();

    let coin_to_send = vault.get_swap_amount();

    let fin_limit_order_sub_msg = create_limit_order_sub_msg(
        existing_pair.address,
        target_price,
        coin_to_send.clone(),
        SUBMIT_ORDER_REPLY_ID,
    );

    let time_trigger_configuration = TimeConfiguration {
        target_time: env.block.time,
        time_interval,
    };

    let fin_limit_order_configuration = FINLimitOrderConfiguration {
        order_idx: Uint128::zero(),
        target_price,
    };

    // removed when trigger change over occurs
    TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID.save(
        deps.storage,
        vault.id.u128(),
        &time_trigger_configuration,
    )?;

    // removed with successful limit order creation
    FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID.save(
        deps.storage,
        vault.id.u128(),
        &fin_limit_order_configuration,
    )?;

    VAULTS.save(deps.storage, (info.sender, vault.id.u128()), &vault)?;

    EXECUTIONS.save(deps.storage, vault.id.into(), &Vec::new())?;

    let cache: Cache = Cache {
        vault_id: vault.id.clone(),
        owner: vault.owner.clone(),
    };
    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_attribute("method", "create_vault_with_fin_limit_order_trigger")
        .add_attribute("id", config.vault_count.to_string())
        .add_attribute("owner", vault.owner)
        .add_attribute("vault_id", vault.id.to_string())
        .add_submessage(fin_limit_order_sub_msg))
}

fn cancel_vault_by_address_and_id(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let validated_address = deps.api.addr_validate(&address)?;
    let vault: Vault<DCAConfiguration, DCAStatus> =
        VAULTS.load(deps.storage, (validated_address.clone(), vault_id.into()))?;
    validate_sender_is_admin_or_vault_owner(deps.as_ref(), vault.owner.clone(), info.sender)?;

    match vault.trigger_variant {
        TriggerVariant::Time => {
            TIME_TRIGGERS.remove(deps.storage, vault.trigger_id.into());
            let balance = vault.get_current_balance().clone();

            let refund_bank_msg = BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![balance.clone()],
            };

            VAULTS.remove(deps.storage, (vault.owner.clone(), vault.id.into()));

            Ok(Response::new()
                .add_attribute("method", "cancel_vault_by_address_and_id")
                .add_attribute("owner", vault.owner.to_string())
                .add_attribute("vault_id", vault.id)
                .add_message(refund_bank_msg))
        }
        TriggerVariant::FINLimitOrder => {
            TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID.remove(deps.storage, vault.id.u128());

            let fin_limit_order_trigger =
                FIN_LIMIT_ORDER_TRIGGERS.load(deps.storage, vault.trigger_id.u128())?;

            let (offer_amount, original_offer_amount, filled) = query_order_details(
                deps.querier,
                vault.configuration.pair.address.clone(),
                fin_limit_order_trigger.configuration.order_idx,
            );

            let limit_order_cache = LimitOrderCache {
                offer_amount,
                original_offer_amount,
                filled,
            };

            LIMIT_ORDER_CACHE.save(deps.storage, &limit_order_cache)?;

            let fin_retract_order_sub_msg = create_retract_order_sub_msg(
                vault.configuration.pair.address,
                fin_limit_order_trigger.configuration.order_idx,
                RETRACT_ORDER_REPLY_ID,
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
    }
}

fn execute_time_trigger_by_id(
    deps: DepsMut,
    env: Env,
    trigger_id: Uint128,
) -> Result<Response, ContractError> {
    let trigger = TIME_TRIGGERS.load(deps.storage, trigger_id.into())?;

    let vault = VAULTS.load(
        deps.storage,
        (trigger.owner.clone(), trigger.vault_id.into()),
    )?;

    // COMMENTED OUT FOR TESTING
    // move this into validation method
    if !target_time_elapsed(env.block.time, trigger.configuration.target_time) {
        return Err(ContractError::CustomError {
            val: String::from("vault execution time has not yet elapsed"),
        });
    }

    // change the status of the vault so frontend knows
    if vault.low_funds() {
        VAULTS.update(
            deps.storage,
            (vault.owner.clone(), vault.id.into()),
            |existing_vault| -> Result<Vault<DCAConfiguration, DCAStatus>, ContractError> {
                match existing_vault {
                    Some(mut existing_vault) => {
                        existing_vault.status = DCAStatus::Inactive;
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
                vault.get_swap_amount(),
                SWAP_REPLY_ID,
            )
        }
        None => create_fin_swap_without_slippage(
            vault.configuration.pair.address.clone(),
            vault.get_swap_amount(),
            SWAP_REPLY_ID,
        ),
    };

    let cache: Cache = Cache {
        vault_id: vault.id,
        owner: vault.owner,
    };
    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_attribute("method", "execute_time_trigger_by_id")
        .add_submessage(fin_swap_msg))
}

fn execute_fin_limit_order_trigger_by_order_idx(
    deps: DepsMut,
    _env: Env,
    order_idx: Uint128,
) -> Result<Response, ContractError> {
    let fin_limit_order_trigger_id =
        FIN_LIMIT_ORDER_TRIGGER_IDS_BY_ORDER_IDX.load(deps.storage, order_idx.u128())?;
    let fin_limit_order_trigger =
        FIN_LIMIT_ORDER_TRIGGERS.load(deps.storage, fin_limit_order_trigger_id.into())?;

    let vault = VAULTS.load(
        deps.storage,
        (
            fin_limit_order_trigger.owner.clone(),
            fin_limit_order_trigger.vault_id.into(),
        ),
    )?;

    let (offer_amount, original_offer_amount, filled) = query_order_details(
        deps.querier,
        vault.configuration.pair.address.clone(),
        order_idx,
    );

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
        vault.configuration.pair.address,
        order_idx,
        EXECUTE_TRIGGER_WITHDRAW_ORDER_REPLY_ID,
    );

    let cache: Cache = Cache {
        vault_id: vault.id,
        owner: vault.owner,
    };
    CACHE.save(deps.storage, &cache)?;

    Ok(Response::new()
        .add_attribute("method", "execute_fin_limit_order_trigger_by_order_idx")
        .add_submessage(fin_withdraw_sub_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        SWAP_REPLY_ID => after_swap(deps, env, reply),
        SUBMIT_ORDER_REPLY_ID => after_submit_order(deps, env, reply),
        EXECUTE_TRIGGER_WITHDRAW_ORDER_REPLY_ID => {
            after_execute_trigger_withdraw_order(deps, env, reply)
        }
        RETRACT_ORDER_REPLY_ID => after_retract_order(deps, env, reply),
        CANCEL_TRIGGER_WITHDRAW_ORDER_REPLY_ID => {
            after_cancel_trigger_withdraw_order(deps, env, reply)
        }
        id => Err(ContractError::CustomError {
            val: format!("unknown reply id: {}", id),
        }),
    }
}

fn after_submit_order(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_submit_order_response = reply.result.into_result().unwrap();

            let wasm_event =
                find_first_event_by_type(&fin_submit_order_response.events, String::from("wasm"))
                    .unwrap();

            let order_idx =
                find_first_attribute_by_key(&wasm_event.attributes, String::from("order_idx"))
                    .unwrap()
                    .value
                    .parse::<Uint128>()
                    .unwrap();

            let cache = CACHE.load(deps.storage)?;
            let fin_limit_order_configuration = FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID
                .load(deps.storage, cache.vault_id.u128())?;

            let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
                config.trigger_count = config.trigger_count.checked_add(Uint128::new(1))?;
                Ok(config)
            })?;

            let fin_limit_order_trigger = TriggerBuilder::from(fin_limit_order_configuration)
                .id(config.trigger_count)
                .owner(cache.owner.clone())
                .vault_id(cache.vault_id)
                .order_idx(order_idx)
                .build();

            VAULTS.update(
                deps.storage,
                (cache.owner.clone(), cache.vault_id.into()),
                |vault| -> Result<Vault<DCAConfiguration, DCAStatus>, ContractError> {
                    match vault {
                        Some(mut existing_vault) => {
                            existing_vault.trigger_id = fin_limit_order_trigger.id;
                            existing_vault.trigger_variant = TriggerVariant::FINLimitOrder;
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

            FIN_LIMIT_ORDER_TRIGGERS.save(
                deps.storage,
                fin_limit_order_trigger.id.u128(),
                &fin_limit_order_trigger,
            )?;

            FIN_LIMIT_ORDER_TRIGGER_IDS_BY_ORDER_IDX.save(
                deps.storage,
                fin_limit_order_trigger.configuration.order_idx.u128(),
                &fin_limit_order_trigger.id.u128(),
            )?;

            FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID.remove(deps.storage, cache.vault_id.u128());

            CACHE.remove(deps.storage);

            Ok(Response::new()
                .add_attribute("method", "after_submit_order")
                .add_attribute("trigger_id", fin_limit_order_trigger.id))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!("failed to create vault with fin limit order trigger: {}", e),
        }),
    }
}

fn after_execute_trigger_withdraw_order(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;
    let vault = VAULTS.load(deps.storage, (cache.owner.clone(), cache.vault_id.into()))?;
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_limit_order_trigger =
                FIN_LIMIT_ORDER_TRIGGERS.load(deps.storage, vault.trigger_id.into())?;
            FIN_LIMIT_ORDER_TRIGGER_IDS_BY_ORDER_IDX.remove(
                deps.storage,
                fin_limit_order_trigger.configuration.order_idx.u128(),
            );
            FIN_LIMIT_ORDER_TRIGGERS.remove(deps.storage, fin_limit_order_trigger.id.u128());

            let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
                config.trigger_count = config.trigger_count.checked_add(Uint128::new(1))?;
                Ok(config)
            })?;

            let time_trigger_configuration =
                TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID.load(deps.storage, vault.id.into())?;

            let time_trigger = TriggerBuilder::from(time_trigger_configuration)
                .id(config.trigger_count)
                .vault_id(vault.id)
                .owner(vault.owner.clone())
                .build();

            TIME_TRIGGERS.save(deps.storage, time_trigger.id.u128(), &time_trigger)?;

            VAULTS.update(
                deps.storage,
                (vault.owner.clone(), vault.id.into()),
                |vault| -> Result<Vault<DCAConfiguration, DCAStatus>, ContractError> {
                    match vault {
                        Some(mut existing_vault) => {
                            existing_vault.balances[0].amount -=
                                limit_order_cache.original_offer_amount;
                            existing_vault.trigger_id = time_trigger.id;
                            existing_vault.trigger_variant = TriggerVariant::Time;
                            if existing_vault.low_funds() {
                                existing_vault.status = DCAStatus::Inactive
                            }
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

            let coin_sent_with_limit_order = Coin {
                denom: vault.get_swap_denom().clone(),
                amount: limit_order_cache.original_offer_amount,
            };

            let coin_received_from_limit_order = Coin {
                denom: vault.get_receive_denom().clone(),
                amount: limit_order_cache.filled,
            };

            let vault_owner_bank_msg: BankMsg = BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![coin_received_from_limit_order.clone()],
            };

            let executions: Vec<Execution<DCAExecutionInformation>> =
                EXECUTIONS.load(deps.storage, vault.id.into())?;

            let number_of_previous_executions: u16 = executions.len().try_into().unwrap();

            let execution = ExecutionBuilder::new()
                .vault_id(vault.id)
                .sequence_id(number_of_previous_executions + 1)
                .block_height(env.block.height)
                .success_fin_limit_order_trigger(
                    coin_sent_with_limit_order,
                    coin_received_from_limit_order,
                )
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
            LIMIT_ORDER_CACHE.remove(deps.storage);
            CACHE.remove(deps.storage);
            Ok(Response::new()
                .add_attribute("method", "after_withdraw_order")
                .add_attribute("trigger_id", time_trigger.id)
                .add_message(vault_owner_bank_msg))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to withdraw fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}

fn after_cancel_trigger_withdraw_order(
    deps: DepsMut,
    _env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = VAULTS.load(deps.storage, (cache.owner.clone(), cache.vault_id.into()))?;
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;

            let fin_limit_order_trigger =
                FIN_LIMIT_ORDER_TRIGGERS.load(deps.storage, vault.trigger_id.into())?;

            // send assets from partially filled order to owner
            let filled_amount = Coin {
                denom: vault.get_receive_denom().clone(),
                amount: limit_order_cache.filled,
            };

            let filled_amount_bank_msg = BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![filled_amount.clone()],
            };

            FIN_LIMIT_ORDER_TRIGGERS.remove(deps.storage, fin_limit_order_trigger.id.u128());
            FIN_LIMIT_ORDER_TRIGGER_IDS_BY_ORDER_IDX.remove(
                deps.storage,
                fin_limit_order_trigger.configuration.order_idx.u128(),
            );

            VAULTS.remove(deps.storage, (vault.owner.clone(), vault.id.into()));

            LIMIT_ORDER_CACHE.remove(deps.storage);
            CACHE.remove(deps.storage);

            Ok(Response::new()
                .add_attribute("method", "after_cancel_trigger_withdraw_order")
                .add_message(filled_amount_bank_msg))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to withdraw fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}

fn after_swap(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = VAULTS.load(deps.storage, (cache.owner.clone(), cache.vault_id.into()))?;
    let trigger: Trigger<TimeConfiguration> =
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
            .parse::<u128>()
            .unwrap();

            let quote_amount = find_first_attribute_by_key(
                &wasm_trade_event.attributes,
                String::from("quote_amount"),
            )
            .unwrap()
            .value
            .parse::<u128>()
            .unwrap();

            let (coin_sent, coin_received) = match vault.configuration.position_type {
                PositionType::Enter => {
                    let sent = Coin {
                        denom: vault.get_swap_denom(),
                        amount: Uint128::from(quote_amount),
                    };
                    let received = Coin {
                        denom: vault.get_receive_denom(),
                        amount: Uint128::from(base_amount),
                    };

                    (sent, received)
                }
                PositionType::Exit => {
                    let sent = Coin {
                        denom: vault.get_swap_denom(),
                        amount: Uint128::from(base_amount),
                    };
                    let received = Coin {
                        denom: vault.get_receive_denom(),
                        amount: Uint128::from(quote_amount),
                    };

                    (sent, received)
                }
            };

            let bank_msg_to_vault_owner: BankMsg = BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![coin_received.clone()],
            };

            messages.push(CosmosMsg::Bank(bank_msg_to_vault_owner));

            VAULTS.update(
                deps.storage,
                (vault.owner.clone(), vault.id.into()),
                |existing_vault| -> Result<Vault<DCAConfiguration, DCAStatus>, ContractError> {
                    match existing_vault {
                        Some(mut existing_vault) => {
                            existing_vault.balances[0].amount -= coin_sent.amount;
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
                .success_time_trigger(coin_sent.clone(), coin_received.clone())
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
        }
        cosmwasm_std::SubMsgResult::Err(e) => {
            let mut execution = ExecutionBuilder::new()
                .vault_id(vault.id)
                .sequence_id(number_of_previous_executions + 1)
                .block_height(env.block.height);

            if e.contains(ERROR_SWAP_SLIPPAGE) {
                execution = execution.fail_slippage();
            } else if e.contains(ERROR_SWAP_INSUFFICIENT_FUNDS) {
                execution = execution.fail_insufficient_funds();
            } else {
                execution = execution.error();
            }

            attributes.push(Attribute::new(
                "status",
                format!(
                    "{:?}",
                    execution.execution_information.clone().unwrap().result
                ),
            ));

            EXECUTIONS.update(deps.storage, vault.id.into(), |existing_executions: Option<Vec<Execution<DCAExecutionInformation>>>| -> Result<Vec<Execution<DCAExecutionInformation>>, ContractError> {
                match existing_executions {
                    Some(mut executions) => {
                        executions.push(execution.build());
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
        }
    };

    CACHE.remove(deps.storage);

    Ok(Response::new()
        .add_attribute("method", "after_execute_vault_by_address_and_id")
        .add_attribute("owner", vault.owner.to_string())
        .add_attribute("vault_id", vault.id)
        .add_attributes(attributes)
        .add_messages(messages))
}

fn after_retract_order(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = VAULTS.load(deps.storage, (cache.owner.clone(), cache.vault_id.into()))?;
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;

            let fin_limit_order_trigger =
                FIN_LIMIT_ORDER_TRIGGERS.load(deps.storage, vault.trigger_id.u128())?;

            let fin_retract_order_response = reply.result.into_result().unwrap();

            let wasm_trade_event =
                find_first_event_by_type(&fin_retract_order_response.events, String::from("wasm"))
                    .unwrap();

            // if this parse method works look to refactor
            let amount_retracted =
                find_first_attribute_by_key(&wasm_trade_event.attributes, String::from("amount"))
                    .unwrap()
                    .value
                    .parse::<Uint128>()
                    .unwrap();

            // if the entire amount isnt retracted, order was partially filled need to send the partially filled assets to user
            if amount_retracted != limit_order_cache.original_offer_amount {
                let retracted_balance = Coin {
                    denom: vault.get_swap_denom().clone(),
                    amount: vault.balances[0].amount
                        - (vault.configuration.swap_amount - amount_retracted),
                };

                let retracted_amount_bank_msg = BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![retracted_balance.clone()],
                };

                let fin_withdraw_sub_msg = create_withdraw_limit_order_sub_msg(
                    vault.configuration.pair.address,
                    fin_limit_order_trigger.configuration.order_idx,
                    CANCEL_TRIGGER_WITHDRAW_ORDER_REPLY_ID,
                );

                Ok(Response::new()
                    .add_attribute("method", "after_retract_order")
                    .add_attribute("withdraw_required", "true")
                    .add_submessage(fin_withdraw_sub_msg)
                    .add_message(retracted_amount_bank_msg))
            } else {
                let balance = vault.get_current_balance();

                let bank_msg = BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![balance.clone()],
                };

                VAULTS.remove(deps.storage, (vault.owner.clone(), vault.id.into()));

                FIN_LIMIT_ORDER_TRIGGERS.remove(deps.storage, fin_limit_order_trigger.id.u128());

                FIN_LIMIT_ORDER_TRIGGER_IDS_BY_ORDER_IDX.remove(
                    deps.storage,
                    fin_limit_order_trigger.configuration.order_idx.u128(),
                );

                LIMIT_ORDER_CACHE.remove(deps.storage);
                CACHE.remove(deps.storage);

                Ok(Response::new()
                    .add_attribute("method", "after_retract_order")
                    .add_attribute("withdraw_required", "false")
                    .add_message(bank_msg))
            }
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to retract fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAllPairs {} => to_binary(&get_all_pairs(deps)?),
        QueryMsg::GetAllTimeTriggers {} => to_binary(&get_all_time_triggers(deps)?),
        QueryMsg::GetAllVaults {} => to_binary(&get_all_vaults(deps)?),
        QueryMsg::GetAllVaultsByAddress { address } => {
            to_binary(&get_all_vaults_by_address(deps, address)?)
        }
        QueryMsg::GetVaultByAddressAndId { address, vault_id } => {
            to_binary(&get_vault_by_address_and_id(deps, address, vault_id)?)
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

fn get_all_time_triggers(deps: Deps) -> StdResult<TriggersResponse<TimeConfiguration>> {
    let all_time_triggers_on_heap: StdResult<Vec<_>> = TIME_TRIGGERS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let triggers: Vec<Trigger<TimeConfiguration>> = all_time_triggers_on_heap
        .unwrap()
        .iter()
        .map(|t| t.1.clone())
        .collect();

    Ok(TriggersResponse { triggers })
}

fn get_all_vaults(deps: Deps) -> StdResult<VaultsResponse> {
    let all_active_vaults_on_heap: StdResult<Vec<_>> = VAULTS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let vaults: Vec<Vault<DCAConfiguration, DCAStatus>> = all_active_vaults_on_heap
        .unwrap()
        .iter()
        .map(|v| v.1.clone())
        .collect();

    Ok(VaultsResponse { vaults })
}

fn get_all_vaults_by_address(deps: Deps, address: String) -> StdResult<VaultsResponse> {
    let validated_address = deps.api.addr_validate(&address)?;

    let vaults_on_heap: StdResult<Vec<_>> = VAULTS
        .prefix(validated_address)
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();

    let vaults: Vec<Vault<DCAConfiguration, DCAStatus>> = vaults_on_heap
        .unwrap()
        .iter()
        .map(|v| -> Vault<DCAConfiguration, DCAStatus> { v.1.clone() })
        .collect();

    Ok(VaultsResponse { vaults })
}

fn get_vault_by_address_and_id(
    deps: Deps,
    address: String,
    vault_id: Uint128,
) -> StdResult<VaultResponse> {
    let validated_address = deps.api.addr_validate(&address)?;
    let vault = VAULTS.load(deps.storage, (validated_address, vault_id.into()))?;
    Ok(VaultResponse { vault })
}

fn get_all_executions_by_vault_id(deps: Deps, vault_id: Uint128) -> StdResult<ExecutionsResponse> {
    let all_executions_on_heap: Vec<Execution<DCAExecutionInformation>> =
        EXECUTIONS.load(deps.storage, vault_id.into())?;

    Ok(ExecutionsResponse {
        executions: all_executions_on_heap,
    })
}
