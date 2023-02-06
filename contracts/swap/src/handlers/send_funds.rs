use cosmwasm_std::{Addr, BankMsg, CosmosMsg, MessageInfo, Response, StdResult};

pub fn send_funds_handler(info: MessageInfo, address: Addr) -> StdResult<Response> {
    Ok(Response::new().add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: address.to_string(),
        amount: info.funds,
    })))
}
