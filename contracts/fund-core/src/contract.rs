use crate::handlers::rebalance::{after_failed_swap_handler, rebalance_handler};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{update_config, Config};
use base::ContractError;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Reply};

use crate::handlers::get_config::get_config_handler;
use crate::handlers::migrate::migrate;

pub type ContractResult<T> = Result<T, ContractError>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    update_config(
        deps.storage,
        &Config {
            router: msg.router.clone(),
            swapper: msg.swapper.clone(),
            base_denom: msg.base_denom.clone(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("router", msg.router.to_string())
        .add_attribute("swapper", msg.swapper.to_string())
        .add_attribute("base_denom", msg.base_denom.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Migrate { new_fund_address } => migrate(deps, env, info, new_fund_address),
        ExecuteMsg::Rebalance {
            allocations,
            slippage_tolerance,
            failure_behaviour,
        } => rebalance_handler(
            deps.as_ref(),
            env,
            info,
            &allocations,
            slippage_tolerance,
            failure_behaviour,
        ),
    }
}

pub const AFTER_FAILED_SWAP_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> ContractResult<Response> {
    match reply.id {
        AFTER_FAILED_SWAP_REPLY_ID => after_failed_swap_handler(),
        id => Err(ContractError::CustomError {
            val: format!("Reply id {} has no after handler", id),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&get_config_handler(deps)?),
    }
}
