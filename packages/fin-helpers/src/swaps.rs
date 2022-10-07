use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, SubMsg, WasmMsg};
use kujira::fin::ExecuteMsg as FINExecuteMsg;

pub fn create_fin_swap_without_slippage(
    pair_address: Addr,
    coin_to_send_with_message: Coin,
    reply_id: u64,
) -> SubMsg {
    let fin_swap_msg = FINExecuteMsg::Swap {
        belief_price: None,
        max_spread: None,
        offer_asset: None,
        to: None,
    };

    let execute_message = WasmMsg::Execute {
        contract_addr: pair_address.to_string(),
        msg: to_binary(&fin_swap_msg).unwrap(),
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

pub fn create_fin_swap_with_slippage(
    pair_address: Addr,
    belief_price: Decimal,
    max_spread: Decimal,
    coin_to_send_with_message: Coin,
    reply_id: u64,
) -> SubMsg {
    let fin_swap_msg = FINExecuteMsg::Swap {
        belief_price: Some(belief_price),
        max_spread: Some(max_spread),
        offer_asset: None,
        to: None,
    };

    let execute_message = WasmMsg::Execute {
        contract_addr: pair_address.to_string(),
        msg: to_binary(&fin_swap_msg).unwrap(),
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
