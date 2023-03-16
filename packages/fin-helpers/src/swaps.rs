use std::str::FromStr;

use crate::queries::query_belief_price;
use base::pair::Pair;
use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, Decimal256, QuerierWrapper, ReplyOn, StdResult, SubMsg, WasmMsg,
};
use kujira::fin::ExecuteMsg;

pub fn create_fin_swap_message(
    querier: QuerierWrapper,
    pair: Pair,
    swap_amount: Coin,
    slippage_tolerance: Option<Decimal256>,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let belief_price = slippage_tolerance
        .map(|_| query_belief_price(querier, &pair, &swap_amount.denom).expect("belief price"));

    let swap_message = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair.address.to_string(),
        msg: to_binary(&ExecuteMsg::Swap {
            belief_price: belief_price
                .map(|price| Decimal256::from_str(&price.to_string()).unwrap()),
            max_spread: slippage_tolerance,
            to: None,
            offer_asset: None,
        })?,
        funds: vec![swap_amount],
    });

    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: swap_message,
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}
