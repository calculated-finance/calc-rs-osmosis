use crate::{constants::SWAP_FEE, queries::query_belief_price};
use base::pool::Pool;
use cosmwasm_std::{Coin, Decimal, QuerierWrapper, ReplyOn, StdResult, SubMsg, Uint128};
use osmosis_std::types::osmosis::{
    gamm::v1beta1::MsgSwapExactAmountIn, poolmanager::v1beta1::SwapAmountInRoute,
};
use std::str::FromStr;

pub fn create_osmosis_swap_message(
    querier: QuerierWrapper,
    sender: String,
    pool: Pool,
    swap_amount: Coin,
    slippage_tolerance: Option<Decimal>,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let token_out_denom = if swap_amount.denom == pool.base_denom {
        pool.quote_denom.clone()
    } else {
        pool.base_denom.clone()
    };

    let token_out_min_amount = slippage_tolerance
        .map_or(Uint128::one(), |slippage_tolerance| {
            let belief_price = query_belief_price(querier, &pool, &swap_amount.denom)
                .expect("belief price of the pool");
            swap_amount.amount
                * (Decimal::one() / belief_price)
                * (Decimal::one() - Decimal::from_str(SWAP_FEE).unwrap())
                * (Decimal::one() - slippage_tolerance)
        })
        .to_string();

    let swap = MsgSwapExactAmountIn {
        sender,
        token_in: Some(swap_amount.clone().into()),
        token_out_min_amount,
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
