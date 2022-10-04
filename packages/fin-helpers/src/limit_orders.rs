use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal256, QuerierWrapper, SubMsg, Uint128, Uint256, WasmMsg,
};
use kujira::fin::{ExecuteMsg as FINExecuteMsg, OrderResponse, QueryMsg as FINQueryMsg};

pub fn create_limit_order_sub_message(
    pair_address: Addr,
    price: Decimal256,
    coin_to_send_with_message: Coin,
    reply_id: u64,
) -> SubMsg {
    let fin_limit_order_msg = FINExecuteMsg::SubmitOrder { price };

    let execute_message = WasmMsg::Execute {
        contract_addr: pair_address.to_string(),
        msg: to_binary(&fin_limit_order_msg).unwrap(),
        funds: vec![coin_to_send_with_message],
    };

    println!("here 1");

    let sub_message = SubMsg {
        id: reply_id,
        msg: CosmosMsg::Wasm(execute_message),
        gas_limit: None,
        reply_on: cosmwasm_std::ReplyOn::Always,
    };

    println!("here 2");

    sub_message
}

pub fn create_withdraw_limit_order_sub_message(
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

pub fn get_fin_order_details(
    querier: QuerierWrapper,
    pair_address: Addr,
    order_idx: Uint128,
) -> (Decimal256, Uint256, Uint256) {
    let fin_order_query_msg = FINQueryMsg::Order { order_idx };
    let order_response: OrderResponse = querier
        .query_wasm_smart(pair_address, &fin_order_query_msg)
        .unwrap();
    (
        order_response.quote_price,
        order_response.original_offer_amount,
        order_response.filled_amount,
    )
}
