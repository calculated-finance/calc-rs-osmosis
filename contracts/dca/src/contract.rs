use crate::error::ContractError;
use crate::handlers::cancel_vault::cancel_vault;
use crate::handlers::create_pair::create_pair;
use crate::handlers::create_vault::create_vault;
use crate::handlers::delete_pair::delete_pair;
use crate::handlers::deposit::deposit;
use crate::handlers::execute_trigger::execute_trigger;
use crate::handlers::fin_limit_order_retracted::fin_limit_order_retracted;
use crate::handlers::fin_limit_order_submitted::fin_limit_order_submitted;
use crate::handlers::fin_limit_order_withdrawn_for_cancel_vault::fin_limit_order_withdrawn_for_cancel_vault;
use crate::handlers::fin_limit_order_withdrawn_for_execute_trigger::fin_limit_order_withdrawn_for_execute_vault;
use crate::handlers::fin_swap_completed::fin_swap_completed;
use crate::handlers::get_events::get_events;
use crate::handlers::get_events_by_resource_id::get_events_by_resource_id;
use crate::handlers::get_pairs::get_pairs;
use crate::handlers::get_time_triggers::get_time_triggers;
use crate::handlers::get_vault::get_vault;
use crate::handlers::get_vaults_by_address::get_vaults_by_address;
use crate::handlers::update_config::update_config;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{
    event_store, trigger_store, vault_store, Config, CACHE, CONFIG,
    FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID, LIMIT_ORDER_CACHE,
    TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;

pub const CONTRACT_NAME: &str = "crates.io:calc-dca";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const FIN_SWAP_COMPLETED_ID: u64 = 1;
pub const FIN_LIMIT_ORDER_SUBMITTED_ID: u64 = 2;
pub const FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_ID: u64 = 3;
pub const FIN_LIMIT_ORDER_RETRACTED_ID: u64 = 4;
pub const FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_ID: u64 = 5;

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    vault_store().clear(deps.storage);
    trigger_store().clear(deps.storage);
    TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID.clear(deps.storage);
    FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID.clear(deps.storage);
    event_store().clear(deps.storage);
    CONFIG.remove(deps.storage);
    CACHE.remove(deps.storage);
    LIMIT_ORDER_CACHE.remove(deps.storage);

    let config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        vault_count: Uint128::zero(),
        fee_collector: deps.api.addr_validate(&msg.fee_collector)?,
        fee_percent: msg.fee_percent,
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
    let config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        vault_count: Uint128::zero(),
        fee_collector: deps.api.addr_validate(&msg.fee_collector)?,
        fee_percent: msg.fee_percent,
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
            pair_address,
            position_type,
            slippage_tolerance,
            swap_amount,
            time_interval,
            target_start_time_utc_seconds,
            target_price,
        } => create_vault(
            deps,
            env,
            info,
            pair_address,
            position_type,
            slippage_tolerance,
            swap_amount,
            time_interval,
            target_start_time_utc_seconds,
            target_price,
        ),
        ExecuteMsg::CancelVault { address, vault_id } => cancel_vault(deps, env, address, vault_id),
        ExecuteMsg::ExecuteTrigger { trigger_id } => execute_trigger(deps, env, trigger_id),
        ExecuteMsg::Deposit { vault_id } => deposit(deps, env, info, vault_id),
        ExecuteMsg::UpdateConfig {
            fee_collector,
            fee_percent,
        } => update_config(deps, info, fee_collector, fee_percent),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        FIN_LIMIT_ORDER_RETRACTED_ID => fin_limit_order_retracted(deps, env, reply),
        FIN_LIMIT_ORDER_SUBMITTED_ID => fin_limit_order_submitted(deps, reply),
        FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_ID => {
            fin_limit_order_withdrawn_for_cancel_vault(deps, env, reply)
        }
        FIN_LIMIT_ORDER_WITHDRAWN_FOR_EXECUTE_VAULT_ID => {
            fin_limit_order_withdrawn_for_execute_vault(deps, env, reply)
        }
        FIN_SWAP_COMPLETED_ID => fin_swap_completed(deps, env, reply),
        id => Err(ContractError::CustomError {
            val: format!("unknown reply id: {}", id),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPairs {} => to_binary(&get_pairs(deps)?),
        QueryMsg::GetTimeTriggers {} => to_binary(&get_time_triggers(deps)?),
        QueryMsg::GetVaultsByAddress { address } => {
            to_binary(&get_vaults_by_address(deps, address)?)
        }
        QueryMsg::GetVault { vault_id } => to_binary(&get_vault(deps, vault_id)?),
        QueryMsg::GetEventsByResourceId { resource_id } => {
            to_binary(&get_events_by_resource_id(deps, resource_id)?)
        }
        QueryMsg::GetEvents { start_after, limit } => {
            to_binary(&get_events(deps, start_after, limit)?)
        }
    }
}
