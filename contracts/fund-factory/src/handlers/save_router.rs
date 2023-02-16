use base::{helpers::message_helpers::get_attribute_in_event, ContractError};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Reply, Response, SubMsg,
    WasmMsg::Instantiate as WasmInstantiate,
};

use crate::{
    contract::AFTER_INSTANTIATE_FUND_REPLY_ID,
    state::{
        cache::{Cache, CACHE},
        config::get_config,
        routers::save_router,
    },
};
use fund_core::msg::InstantiateMsg as FundInstantiateMsg;

pub fn save_router_handler(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;

    let instantiate_router_response = reply.result.unwrap();

    let router_address = Addr::unchecked(get_attribute_in_event(
        &instantiate_router_response.events,
        "instantiate",
        "_contract_address",
    )?);

    save_router(
        deps.storage,
        cache
            .owner
            .clone()
            .expect("an owner is set in previous logic"),
        router_address.clone(),
    )?;

    CACHE.save(
        deps.storage,
        &Cache {
            owner: cache.owner,
            router_address: Some(router_address.clone()),
        },
    )?;

    let config = get_config(deps.storage)?;

    let fund_instantiate_msg = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmInstantiate {
            admin: None,
            label: format!("CALC-MF-FUND"),
            code_id: config.fund_code_id,
            funds: vec![],
            msg: to_binary(&FundInstantiateMsg {
                router: Addr::unchecked("router"),
                swapper: Addr::unchecked("swapper"),
                base_denom: String::from("uusd"),
            })?,
        }),
        AFTER_INSTANTIATE_FUND_REPLY_ID,
    );

    Ok(Response::new()
        .add_attribute("method", "save_router_handler")
        .add_attribute("router_address", router_address)
        .add_submessage(fund_instantiate_msg))
}
