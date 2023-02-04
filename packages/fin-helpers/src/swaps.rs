use crate::{
    position_type::PositionType,
    queries::{query_base_price, query_quote_price},
};
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
    let belief_price = slippage_tolerance.map(|_| {
        let position_type = match swap_amount.denom == pair.quote_denom {
            true => PositionType::Enter,
            false => PositionType::Exit,
        };

        let fin_price = match position_type {
            PositionType::Enter => query_base_price(querier, pair.address.clone()),
            PositionType::Exit => query_quote_price(querier, pair.address.clone()),
        };

        match position_type {
            PositionType::Enter => fin_price,
            PositionType::Exit => Decimal256::one()
                .checked_div(fin_price)
                .expect("should return a valid inverted price for fin sell"),
        }
    });

    let swap_message = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: pair.address.to_string(),
        msg: to_binary(&ExecuteMsg::Swap {
            belief_price,
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
