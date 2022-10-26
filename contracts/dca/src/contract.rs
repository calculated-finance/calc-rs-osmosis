use crate::error::ContractError;
use crate::handlers::after_fin_limit_order_retracted::after_fin_limit_order_retracted;
use crate::handlers::after_fin_limit_order_submitted::after_fin_limit_order_submitted;
use crate::handlers::after_fin_limit_order_withdrawn_for_cancel_vault::after_fin_limit_order_withdrawn_for_cancel_vault;
use crate::handlers::after_fin_limit_order_withdrawn_for_execute_trigger::after_fin_limit_order_withdrawn_for_execute_vault;
use crate::handlers::after_fin_swap::after_fin_swap;
use crate::handlers::after_z_delegation::after_z_delegation;
use crate::handlers::cancel_vault::cancel_vault;
use crate::handlers::create_pair::create_pair;
use crate::handlers::create_vault::create_vault;
use crate::handlers::delete_pair::delete_pair;
use crate::handlers::deposit::deposit;
use crate::handlers::execute_trigger::execute_trigger;
use crate::handlers::get_events::get_events;
use crate::handlers::get_events_by_resource_id::get_events_by_resource_id;
use crate::handlers::get_pairs::get_pairs;
use crate::handlers::get_time_trigger_ids::get_time_trigger_ids;
use crate::handlers::get_trigger_id_by_fin_limit_order_idx::get_trigger_id_by_fin_limit_order_idx;
use crate::handlers::get_vault::get_vault;
use crate::handlers::get_vaults_by_address::get_vaults_by_address;
use crate::handlers::update_config::update_config;
use crate::handlers::update_vault_label::update_vault_label;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{clear_triggers, clear_vaults, event_store, Config, CONFIG, VAULT_COUNTER};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;

pub const CONTRACT_NAME: &str = "crates.io:calc-dca";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const AFTER_FIN_SWAP_REPLY_ID: u64 = 1;
pub const AFTER_FIN_LIMIT_ORDER_SUBMITTED_REPLY_ID: u64 = 2;
pub const AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID: u64 = 3;
pub const AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID: u64 = 4;
pub const AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID: u64 = 5;
pub const AFTER_Z_DELEGATION_REPLY_ID: u64 = 6;

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    clear_vaults(deps.storage);
    clear_triggers(deps.storage);
    event_store().clear(deps.storage);
    VAULT_COUNTER.remove(deps.storage);
    CONFIG.remove(deps.storage);

    deps.api.addr_validate(&msg.admin.to_string())?;
    deps.api.addr_validate(&msg.fee_collector.to_string())?;
    deps.api
        .addr_validate(&msg.staking_router_address.to_string())?;

    let config = Config {
        admin: msg.admin,
        fee_collector: msg.fee_collector,
        fee_percent: msg.fee_percent,
        staking_router_address: msg.staking_router_address,
    };

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&msg.admin.to_string())?;
    deps.api.addr_validate(&msg.fee_collector.to_string())?;
    deps.api
        .addr_validate(&msg.staking_router_address.to_string())?;

    let config = Config {
        admin: msg.admin.clone(),
        fee_collector: msg.fee_collector,
        fee_percent: msg.fee_percent,
        staking_router_address: msg.staking_router_address,
    };

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
        ExecuteMsg::CreateVault {
            label,
            destinations,
            pair_address,
            position_type,
            slippage_tolerance,
            price_threshold,
            swap_amount,
            time_interval,
            target_start_time_utc_seconds,
            target_price,
        } => create_vault(
            deps,
            env,
            info,
            label,
            destinations.unwrap_or(vec![]),
            pair_address,
            position_type,
            slippage_tolerance,
            price_threshold,
            swap_amount,
            time_interval,
            target_start_time_utc_seconds,
            target_price,
        ),
        ExecuteMsg::CancelVault { address, vault_id } => cancel_vault(deps, env, address, vault_id),
        ExecuteMsg::ExecuteTrigger { trigger_id } => execute_trigger(deps, env, trigger_id),
        ExecuteMsg::Deposit { address, vault_id } => deposit(deps, env, info, address, vault_id),
        ExecuteMsg::UpdateConfig {
            fee_collector,
            fee_percent,
            staking_router_address,
        } => update_config(
            deps,
            info,
            fee_collector,
            fee_percent,
            staking_router_address,
        ),
        ExecuteMsg::UpdateVault {
            address,
            vault_id,
            label,
        } => update_vault_label(deps, info, address, vault_id, label),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        AFTER_FIN_LIMIT_ORDER_RETRACTED_REPLY_ID => {
            after_fin_limit_order_retracted(deps, env, reply)
        }
        AFTER_FIN_LIMIT_ORDER_SUBMITTED_REPLY_ID => after_fin_limit_order_submitted(deps, reply),
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID => {
            after_fin_limit_order_withdrawn_for_cancel_vault(deps, env, reply)
        }
        AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_REPLY_ID => {
            after_fin_limit_order_withdrawn_for_execute_vault(deps, env, reply)
        }
        AFTER_FIN_SWAP_REPLY_ID => after_fin_swap(deps, env, reply),
        AFTER_Z_DELEGATION_REPLY_ID => after_z_delegation(deps, env, reply),
        id => Err(ContractError::CustomError {
            val: format!("unknown reply id: {}", id),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPairs {} => to_binary(&get_pairs(deps)?),
        QueryMsg::GetTimeTriggerIds {} => to_binary(&get_time_trigger_ids(deps, env)?),
        QueryMsg::GetTriggerIdByFinLimitOrderIdx { order_idx } => {
            to_binary(&get_trigger_id_by_fin_limit_order_idx(deps, order_idx)?)
        }
        QueryMsg::GetVaultsByAddress {
            address,
            start_after,
            limit,
        } => to_binary(&get_vaults_by_address(deps, address, start_after, limit)?),
        QueryMsg::GetVault { vault_id, address } => to_binary(&get_vault(deps, address, vault_id)?),
        QueryMsg::GetEventsByResourceId { resource_id } => {
            to_binary(&get_events_by_resource_id(deps, resource_id)?)
        }
        QueryMsg::GetEvents { start_after, limit } => {
            to_binary(&get_events(deps, start_after, limit)?)
        }
    }
}
