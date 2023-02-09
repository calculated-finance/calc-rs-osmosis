use base::{helpers::message_helpers::get_attribute_in_event, ContractError};
use cosmwasm_std::{Addr, DepsMut, Reply, Response};

use crate::state::{cache::CACHE, fund_routers::save_fund_router};

pub fn save_fund_router_address(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;

    let instantiate_fund_router_response = reply.result.unwrap();

    let fund_router_address = Addr::unchecked(get_attribute_in_event(
        &instantiate_fund_router_response.events,
        "instantiate",
        "_contract_address",
    )?);

    save_fund_router(deps.storage, cache.owner, fund_router_address.clone())?;

    Ok(Response::new()
        .add_attribute("method", "save_fund_router_address")
        .add_attribute("fund_router_address", fund_router_address))
}
