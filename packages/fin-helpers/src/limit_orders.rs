use cosmwasm_std::{SubMsg, CosmosMsg, Decimal256, WasmMsg, Addr, to_binary, Coin};
use kujira::fin::ExecuteMsg as FINExecuteMsg;

pub fn create_limit_order_sub_msg(
    pair_address: Addr,
    price: Decimal256,
    coin_to_send_with_message: Coin,
    reply_id: u64
) -> SubMsg {
    let fin_limit_order_msg = FINExecuteMsg::SubmitOrder {
         price
    };

    let execute_message = WasmMsg::Execute { 
        contract_addr: pair_address.to_string(),
        msg: to_binary(&fin_limit_order_msg).unwrap(), 
        funds: vec![coin_to_send_with_message]
    };

    let sub_message = SubMsg {
        id: reply_id,
        msg: CosmosMsg::Wasm(execute_message),
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Always
    };

    sub_message
}