use super::{price::query_belief_price, routes::calculate_route};
use crate::types::pair::Pair;
use cosmwasm_std::{Coin, Decimal, Env, QuerierWrapper, ReplyOn, StdResult, SubMsg, Uint128};
use osmosis_std::types::osmosis::poolmanager::v1beta1::MsgSwapExactAmountIn;

pub fn create_osmosis_swap_message(
    querier: &QuerierWrapper,
    env: &Env,
    pair: &Pair,
    swap_amount: Coin,
    slippage_tolerance: Option<Decimal>,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let routes = calculate_route(querier, pair, swap_amount.denom.clone())?;

    let token_out_min_amount = match slippage_tolerance {
        Some(slippage_tolerance) => {
            let belief_price = query_belief_price(querier, pair, swap_amount.denom.clone())?;

            swap_amount.amount
                * (Decimal::one() / belief_price)
                * (Decimal::one() - slippage_tolerance)
        }
        _ => Uint128::one(),
    };

    let msg = MsgSwapExactAmountIn {
        sender: env.contract.address.to_string(),
        token_in: Some(swap_amount.into()),
        token_out_min_amount: token_out_min_amount.to_string(),
        routes,
    };

    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: msg.into(),
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}
