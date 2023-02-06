use cosmwasm_std::{Addr, BankMsg, CosmosMsg, MessageInfo, Response};

use crate::contract::ContractResult;

pub fn send_funds_handler(info: MessageInfo, address: Addr) -> ContractResult<Response> {
    Ok(Response::new().add_message(CosmosMsg::Bank(BankMsg::Send {
        to_address: address.to_string(),
        amount: info.funds,
    })))
}
