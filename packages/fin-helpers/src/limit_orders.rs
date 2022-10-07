use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, SubMsg, Uint128, WasmMsg};
use kujira::fin::ExecuteMsg as FINExecuteMsg;

pub fn create_limit_order_sub_msg(
    pair_address: Addr,
    price: Decimal,
    coin_to_send_with_message: Coin,
    reply_id: u64,
) -> SubMsg {
    let fin_limit_order_msg = FINExecuteMsg::SubmitOrder { price };

    let execute_message = WasmMsg::Execute {
        contract_addr: pair_address.to_string(),
        msg: to_binary(&fin_limit_order_msg).unwrap(),
        funds: vec![coin_to_send_with_message],
    };

    let sub_message = SubMsg {
        id: reply_id,
        msg: CosmosMsg::Wasm(execute_message),
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Always,
    };

    sub_message
}

pub fn create_withdraw_limit_order_sub_msg(
    pair_address: Addr,
    order_idx: Uint128,
    reply_id: u64,
) -> SubMsg {
    let fin_withdraw_order_msg = FINExecuteMsg::WithdrawOrders {
        order_idxs: Some(vec![order_idx]),
    };

    let execute_message = WasmMsg::Execute {
        contract_addr: pair_address.to_string(),
        msg: to_binary(&fin_withdraw_order_msg).unwrap(),
        funds: vec![],
    };

    let sub_message = SubMsg {
        id: reply_id,
        msg: CosmosMsg::Wasm(execute_message),
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Always,
    };

    sub_message
}

pub fn create_retract_order_sub_msg(
    pair_address: Addr,
    order_idx: Uint128,
    reply_id: u64,
) -> SubMsg {
    let fin_retract_order_msg = FINExecuteMsg::RetractOrder {
        order_idx,
        amount: None,
    };

    let execute_message = WasmMsg::Execute {
        contract_addr: pair_address.to_string(),
        msg: to_binary(&fin_retract_order_msg).unwrap(),
        funds: vec![],
    };

    let sub_message = SubMsg {
        id: reply_id,
        msg: CosmosMsg::Wasm(execute_message),
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Always,
    };

    sub_message
}
