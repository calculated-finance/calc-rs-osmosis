#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;

use crate::handlers::create_fund_router::create_fund_router;
use crate::handlers::get_config::get_config_handler;
use crate::handlers::get_fund_routers_by_address::get_fund_routers_by_address_handler;
use crate::handlers::save_fund_router_address::save_fund_router_address;
use crate::handlers::update_config::update_config_handler;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::config::{update_config, Config};

use base::ContractError;

const CONTRACT_NAME: &str = "crates.io:fund-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
            admin: msg.admin,
            fund_router_code_id: msg.fund_router_code_id,
            fund_core_code_id: msg.fund_core_code_id,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateManagedFund { token_name } => {
            create_fund_router(deps, env, info, token_name)
        }
        ExecuteMsg::UpdateConfig {
            admin,
            fund_router_code_id,
            fund_core_code_id,
        } => update_config_handler(deps, info, admin, fund_router_code_id, fund_core_code_id),
    }
}

pub const AFTER_INSTANTIATE_FUND_ROUTER_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        AFTER_INSTANTIATE_FUND_ROUTER_REPLY_ID => save_fund_router_address(deps, reply),
        id => Err(ContractError::CustomError {
            val: format!("unknown reply id: {}", id),
        }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&get_config_handler(deps)?),
        QueryMsg::GetFundRouters { owner } => {
            to_binary(&get_fund_routers_by_address_handler(deps, owner)?)
        }
    }
}
