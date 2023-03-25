use base::pool::Pool;
use cosmwasm_std::{Coin, Decimal, QuerierWrapper, ReplyOn, StdResult, SubMsg, Uint128};
use osmosis_std::types::osmosis::{
    gamm::v1beta1::MsgSwapExactAmountIn, poolmanager::v1beta1::SwapAmountInRoute,
};
pub fn create_osmosis_swap_message(
    _querier: QuerierWrapper,
    sender: String,
    pool: Pool,
    swap_amount: Coin,
    _slippage_tolerance: Option<Decimal>,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
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

    let slippage = Uint128::one();

    let token_out_denom = if swap_amount.denom == pool.base_denom {
        pool.quote_denom
    } else {
        pool.base_denom
    };

    let swap = MsgSwapExactAmountIn {
        sender,
        token_in: Some(swap_amount.into()),
        token_out_min_amount: slippage.into(),
        routes: vec![SwapAmountInRoute {
            pool_id: pool.pool_id,
            token_out_denom,
        }],
    };

    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: swap.into(),
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}
