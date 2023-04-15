use crate::error::ContractError;
use crate::handlers::after_z_delegation::after_z_delegation;
use crate::handlers::cancel_vault::cancel_vault;
use crate::handlers::create_custom_swap_fee::create_custom_swap_fee;
use crate::handlers::create_pair::create_pair;
use crate::handlers::create_vault::create_vault_handler;
use crate::handlers::delete_pair::delete_pair;
use crate::handlers::deposit::deposit;
use crate::handlers::disburse_escrow::disburse_escrow;
use crate::handlers::disburse_funds::disburse_funds;
use crate::handlers::execute_trigger::execute_trigger_handler;
use crate::handlers::get_custom_swap_fees::get_custom_swap_fees;
use crate::handlers::get_dca_plus_performance::get_dca_plus_performance_handler;
use crate::handlers::get_disburse_escrow_tasks::get_disburse_escrow_tasks_handler;
use crate::handlers::get_events::get_events;
use crate::handlers::get_events_by_resource_id::get_events_by_resource_id;
use crate::handlers::get_pairs::get_pairs;
use crate::handlers::get_time_trigger_ids::get_time_trigger_ids;
use crate::handlers::get_vault::get_vault;
use crate::handlers::get_vaults::get_vaults_handler;
use crate::handlers::get_vaults_by_address::get_vaults_by_address;
use crate::handlers::remove_custom_swap_fee::remove_custom_swap_fee;
use crate::handlers::update_config::update_config_handler;
use crate::handlers::update_swap_adjustments_handler::update_swap_adjustments_handler;
use crate::handlers::z_provide_liquidity::{
    bond_lp_tokens, log_bond_lp_tokens_result, provide_liquidity_handler, send_lp_tokens,
};
use crate::helpers::validation_helpers::{
    assert_dca_plus_escrow_level_is_less_than_100_percent,
    assert_fee_collector_addresses_are_valid, assert_fee_collector_allocations_add_up_to_one,
};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::config::{get_config, update_config, Config};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;

pub const CONTRACT_NAME: &str = "crates.io:calc-dca";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.addr_validate(&msg.admin.to_string())?;
    deps.api
        .addr_validate(&msg.staking_router_address.to_string())?;

    assert_fee_collector_addresses_are_valid(deps.as_ref(), &msg.fee_collectors)?;
    assert_fee_collector_allocations_add_up_to_one(&msg.fee_collectors)?;
    assert_dca_plus_escrow_level_is_less_than_100_percent(msg.dca_plus_escrow_level)?;

    update_config(
        deps.storage,
        Config {
            admin: msg.admin.clone(),
            fee_collectors: msg.fee_collectors,
            swap_fee_percent: msg.swap_fee_percent,
            delegation_fee_percent: msg.delegation_fee_percent,
            staking_router_address: msg.staking_router_address,
            page_limit: msg.page_limit,
            paused: msg.paused,
            dca_plus_escrow_level: msg.dca_plus_escrow_level,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&msg.admin.to_string())?;
    deps.api
        .addr_validate(&msg.staking_router_address.to_string())?;

    assert_fee_collector_addresses_are_valid(deps.as_ref(), &msg.fee_collectors)?;
    assert_fee_collector_allocations_add_up_to_one(&msg.fee_collectors)?;
    assert_dca_plus_escrow_level_is_less_than_100_percent(msg.dca_plus_escrow_level)?;

    update_config(
        deps.storage,
        Config {
            admin: msg.admin.clone(),
            fee_collectors: msg.fee_collectors,
            swap_fee_percent: msg.swap_fee_percent,
            delegation_fee_percent: msg.delegation_fee_percent,
            staking_router_address: msg.staking_router_address,
            page_limit: msg.page_limit,
            paused: msg.paused,
            dca_plus_escrow_level: msg.dca_plus_escrow_level,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

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
            route,
        } => create_pair(deps, info, address, base_denom, quote_denom, route),
        ExecuteMsg::DeletePair { address } => delete_pair(deps, info, address),
        ExecuteMsg::CreateVault {
            owner,
            label,
            destinations,
            pair_address,
            position_type,
            slippage_tolerance,
            minimum_receive_amount,
            swap_amount,
            time_interval,
            target_start_time_utc_seconds,
            use_dca_plus,
        } => create_vault_handler(
            deps,
            env,
            &info,
            owner.unwrap_or(info.sender.clone()),
            label,
            destinations.unwrap_or(vec![]),
            pair_address,
            position_type,
            slippage_tolerance,
            minimum_receive_amount,
            swap_amount,
            time_interval,
            target_start_time_utc_seconds,
            use_dca_plus,
        ),
        ExecuteMsg::CancelVault { vault_id } => cancel_vault(deps, env, info, vault_id),
        ExecuteMsg::ExecuteTrigger { trigger_id } => execute_trigger_handler(deps, env, trigger_id),
        ExecuteMsg::Deposit { address, vault_id } => deposit(deps, env, info, address, vault_id),
        ExecuteMsg::UpdateConfig {
            fee_collectors,
            swap_fee_percent,
            delegation_fee_percent,
            staking_router_address,
            page_limit,
            paused,
            dca_plus_escrow_level,
        } => update_config_handler(
            deps,
            info,
            fee_collectors,
            swap_fee_percent,
            delegation_fee_percent,
            staking_router_address,
            page_limit,
            paused,
            dca_plus_escrow_level,
        ),
        ExecuteMsg::CreateCustomSwapFee {
            denom,
            swap_fee_percent,
        } => create_custom_swap_fee(deps, info, denom, swap_fee_percent),
        ExecuteMsg::RemoveCustomSwapFee { denom } => remove_custom_swap_fee(deps, info, denom),
        ExecuteMsg::UpdateSwapAdjustments {
            position_type,
            adjustments,
        } => update_swap_adjustments_handler(deps, env, position_type, adjustments),
        ExecuteMsg::DisburseEscrow { vault_id } => disburse_escrow(deps, &env, info, vault_id),
        ExecuteMsg::ProvideLiquidity {
            provider_address,
            pool_id,
            duration,
            slippage_tolerance,
        } => provide_liquidity_handler(
            deps,
            env,
            info,
            provider_address,
            pool_id,
            duration,
            slippage_tolerance,
        ),
    }
}

pub const AFTER_SWAP_REPLY_ID: u64 = 1;
pub const AFTER_Z_DELEGATION_REPLY_ID: u64 = 2;
pub const AFTER_PROVIDE_LIQUIDITY_REPLY_ID: u64 = 3;
pub const AFTER_SEND_LP_TOKENS_REPLY_ID: u64 = 4;
pub const AFTER_BOND_LP_TOKENS_REPLY_ID: u64 = 5;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        AFTER_SWAP_REPLY_ID => disburse_funds(deps, &env, reply),
        AFTER_Z_DELEGATION_REPLY_ID => after_z_delegation(deps, env, reply),
        AFTER_PROVIDE_LIQUIDITY_REPLY_ID => send_lp_tokens(deps, env),
        AFTER_SEND_LP_TOKENS_REPLY_ID => bond_lp_tokens(deps.as_ref(), env),
        AFTER_BOND_LP_TOKENS_REPLY_ID => log_bond_lp_tokens_result(deps, reply),
        id => Err(ContractError::CustomError {
            val: format!("unhandled DCA contract reply id: {}", id),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPairs {} => to_binary(&get_pairs(deps)?),
        QueryMsg::GetTimeTriggerIds { limit } => {
            to_binary(&get_time_trigger_ids(deps, env, limit)?)
        }
        QueryMsg::GetVaults { start_after, limit } => {
            to_binary(&get_vaults_handler(deps, start_after, limit)?)
        }
        QueryMsg::GetVaultsByAddress {
            address,
            status,
            start_after,
            limit,
        } => to_binary(&get_vaults_by_address(
            deps,
            address,
            status,
            start_after,
            limit,
        )?),
        QueryMsg::GetVault { vault_id } => to_binary(&get_vault(deps, vault_id)?),
        QueryMsg::GetEventsByResourceId {
            resource_id,
            start_after,
            limit,
        } => to_binary(&get_events_by_resource_id(
            deps,
            resource_id,
            start_after,
            limit,
        )?),
        QueryMsg::GetEvents { start_after, limit } => {
            to_binary(&get_events(deps, start_after, limit)?)
        }
        QueryMsg::GetCustomSwapFees {} => to_binary(&get_custom_swap_fees(deps)?),
        QueryMsg::GetConfig {} => to_binary(&ConfigResponse {
            config: get_config(deps.storage)?,
        }),
        QueryMsg::GetDcaPlusPerformance { vault_id } => {
            to_binary(&get_dca_plus_performance_handler(deps, vault_id)?)
        }
        QueryMsg::GetDisburseEscrowTasks { limit } => {
            to_binary(&get_disburse_escrow_tasks_handler(deps, env, limit)?)
        }
    }
}
