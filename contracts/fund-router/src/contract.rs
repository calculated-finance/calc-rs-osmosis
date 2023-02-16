use base::ContractError;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg,
};
use kujira::denom::Denom;
use kujira::msg::{DenomMsg, KujiraMsg};

use crate::handlers::assign_fund::assign_fund;
use crate::handlers::get_config::get_config_handler;
use crate::handlers::get_fund::get_fund;
use crate::handlers::get_fund_config::get_fund_config;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::config::{update_config, Config};
use crate::state::funds::initialise_funds;

pub const AFTER_INSTANTIATE_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    initialise_funds(deps.storage)?;

    update_config(
        deps.storage,
        Config {
            factory: info.sender.clone(),
            owner: msg.owner,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_submessage(SubMsg::new(DenomMsg::Create {
            subdenom: Denom::from(msg.token_name),
        })))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AssignFund { fund_address } => assign_fund(deps, info, fund_address),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetFund {} => to_binary(&get_fund(deps)?),
        QueryMsg::GetFundConfig {} => to_binary(&get_fund_config(deps)?),
        QueryMsg::GetConfig {} => to_binary(&get_config_handler(deps)?),
    }
}
