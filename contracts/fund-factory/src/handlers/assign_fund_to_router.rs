use crate::state::cache::CACHE;
use base::{helpers::message_helpers::get_attribute_in_event, ContractError};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Reply, Response, SubMsg,
    WasmMsg::Execute as WasmExecuteMsg,
};
use fund_router::msg::ExecuteMsg as RouterExecuteMsg;

pub fn assign_fund_to_router(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;

    let instantiate_fund_response = reply.result.unwrap();

    let fund_address = Addr::unchecked(get_attribute_in_event(
        &instantiate_fund_response.events,
        "instantiate",
        "_contract_address",
    )?);

    let assign_fund_msg = SubMsg::new(CosmosMsg::Wasm(WasmExecuteMsg {
        contract_addr: cache
            .router_address
            .expect("router address is set in previous logic")
            .to_string(),
        funds: vec![],
        msg: to_binary(&RouterExecuteMsg::AssignFund {
            fund_address: fund_address.clone(),
        })?,
    }));

    Ok(Response::new()
        .add_attribute("method", "assign_fund_to_router")
        .add_attribute("fund_address", fund_address)
        .add_submessage(assign_fund_msg))
}
