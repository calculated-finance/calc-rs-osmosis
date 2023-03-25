use base::pool::Pool;
use cosmwasm_std::{
    Coin, Decimal, QuerierWrapper, ReplyOn, StdResult, SubMsg,
};

pub fn create_fin_swap_message(
    _querier: QuerierWrapper,
    _pool: Pool,
    _swap_amount: Coin,
    _slippage_tolerance: Option<Decimal>,
    _reply_id: Option<u64>,
    _reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    // let belief_price = slippage_tolerance
    //     .map(|_| query_belief_price(querier, &pair, &swap_amount.denom).expect("belief price"));

    // let swap_message = CosmosMsg::Wasm(WasmMsg::Execute {
    //     contract_addr: pair.pool_id.to_string(),
    //     msg: to_binary(&ExecuteMsg::Swap {
    //         belief_price: belief_price
    //             .map(|belief_price| Decimal256::from_str(&belief_price.to_string()).unwrap()),
    //         max_spread: slippage_tolerance.map(|slippage_tolerance| {
    //             Decimal256::from_str(&slippage_tolerance.to_string()).unwrap()
    //         }),
    //         to: None,
    //         offer_asset: None,
    //     })?,
    //     funds: vec![swap_amount],
    // });

    // Ok(SubMsg {
    //     id: reply_id.unwrap_or(0),
    //     msg: swap_message,
    //     gas_limit: None,
    //     reply_on: reply_on.unwrap_or(ReplyOn::Never),
    // })
    unimplemented!()
}
