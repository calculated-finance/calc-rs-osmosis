use crate::{
    errors::contract_error::ContractError,
    handlers::{
        add_path::add_path_handler,
        continue_swap::continue_swap_handler,
        create_swap::create_swap_handler,
        send_funds::send_funds_handler,
        swap_on_fin::{after_swap_on_fin_handler, swap_on_fin_handler},
        update_config::update_config_handler,
    },
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        config::{get_config, update_config, Config},
        paths::get_path,
    },
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;

pub const CONTRACT_NAME: &str = "crates.io:calc-swap";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn migrate(_: DepsMut, _: Env, _: MigrateMsg) -> Result<Response, ContractError> {
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

    update_config(
        deps.storage,
        Config {
            admin: msg.admin.clone(),
            paused: false,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { admin, paused } => {
            update_config_handler(deps, info, Config { admin, paused })
        }
        ExecuteMsg::AddPath { pair } => add_path_handler(deps, info, pair),
        ExecuteMsg::CreateSwap {
            target_denom,
            slippage_tolerance,
            on_complete,
        } => create_swap_handler(
            deps,
            env,
            info,
            target_denom,
            slippage_tolerance,
            on_complete,
        ),
        ExecuteMsg::ContinueSwap { swap_id } => continue_swap_handler(deps, info, swap_id),
        ExecuteMsg::SwapOnFin {
            pair,
            slippage_tolerance,
            callback,
        } => swap_on_fin_handler(deps, &env, &info, pair, slippage_tolerance, callback),
        ExecuteMsg::SendFunds { address } => send_funds_handler(info, address),
    }
}

pub const AFTER_FIN_SWAP_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        AFTER_FIN_SWAP_REPLY_ID => after_swap_on_fin_handler(deps, env),
        id => Err(StdError::generic_err(format!(
            "Reply id {} has no after handler",
            id
        ))),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&get_config(deps.storage)?),
        QueryMsg::GetPath { denoms } => to_binary(&get_path(deps.storage, denoms)?),
    }
}
