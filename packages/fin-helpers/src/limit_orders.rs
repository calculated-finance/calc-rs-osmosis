use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal256, SubMsg, Uint128, WasmMsg};
use kujira::fin::ExecuteMsg as FINExecuteMsg;

pub fn create_limit_order_sub_msg(
    pair_address: Addr,
    price: Decimal256,
    coin_to_send_with_message: Coin,
    reply_id: u64,
) -> SubMsg {
    let fin_limit_order_msg = FINExecuteMsg::SubmitOrder { price };

    SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_address.to_string(),
            msg: to_binary(&fin_limit_order_msg).unwrap(),
            funds: vec![coin_to_send_with_message],
        }),
        reply_id,
    )
}

pub fn create_withdraw_limit_order_sub_msg(
    pair_address: Addr,
    order_idx: Uint128,
    reply_id: u64,
) -> SubMsg {
    let fin_withdraw_order_msg = FINExecuteMsg::WithdrawOrders {
        order_idxs: Some(vec![order_idx]),
    };

    SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_address.to_string(),
            msg: to_binary(&fin_withdraw_order_msg).unwrap(),
            funds: vec![],
        }),
        reply_id,
    )
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

    SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_address.to_string(),
            msg: to_binary(&fin_retract_order_msg).unwrap(),
            funds: vec![],
        }),
        reply_id,
    )
}
