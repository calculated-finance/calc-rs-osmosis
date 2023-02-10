use crate::state::cache::{Cache, CACHE};
use crate::{contract::AFTER_INSTANTIATE_FUND_ROUTER_REPLY_ID, state::config::get_config};
use base::ContractError;
use cosmwasm_std::WasmMsg::Instantiate as WasmInstantiate;
use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, SubMsg};
use fund_router::msg::InstantiateMsg as RouterInstantiateMsg;

pub fn create_fund_router(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_name: String,
) -> Result<Response, ContractError> {
    let config = get_config(deps.storage)?;

    let fund_router_instantiate_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmInstantiate {
            admin: None,
            label: format!("CALC-MF-ROUTER"),
            code_id: config.fund_router_code_id,
            funds: vec![],
            msg: to_binary(&RouterInstantiateMsg {
                token_name: token_name.clone(),
            })?,
        }),
        AFTER_INSTANTIATE_FUND_ROUTER_REPLY_ID,
    );

    CACHE.save(deps.storage, &Cache { owner: info.sender })?;

    Ok(Response::new()
        .add_attribute("method", "create_managed_fund")
        .add_submessage(fund_router_instantiate_msg))
}
