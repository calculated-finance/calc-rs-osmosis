use crate::state::cache::{Cache, CACHE};
use crate::{contract::AFTER_INSTANTIATE_ROUTER_REPLY_ID, state::config::get_config};
use base::ContractError;
use cosmwasm_std::WasmMsg::Instantiate as WasmInstantiate;
use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, SubMsg};
use fund_router::msg::InstantiateMsg as RouterInstantiateMsg;

pub fn create_router(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_name: String,
) -> Result<Response, ContractError> {
    let config = get_config(deps.storage)?;

    let router_instantiate_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmInstantiate {
            admin: None,
            label: format!("CALC-MF-ROUTER"),
            code_id: config.router_code_id,
            funds: vec![info.funds[0].clone()],
            msg: to_binary(&RouterInstantiateMsg {
                token_name: token_name.clone(),
            })?,
        }),
        AFTER_INSTANTIATE_ROUTER_REPLY_ID,
    );

    CACHE.save(
        deps.storage,
        &Cache {
            owner: info.sender,
            router_address: None,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "create_router")
        .add_submessage(router_instantiate_msg))
}
