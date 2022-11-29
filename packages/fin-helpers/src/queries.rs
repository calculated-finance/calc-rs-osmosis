use cosmwasm_std::{Addr, Decimal256, QuerierWrapper, StdResult, Uint128};
use kujira::fin::QueryMsg as FINQueryMsg;

use crate::msg::{FINBookResponse, FINOrderResponseWithoutDenom};

pub fn query_base_price(querier: QuerierWrapper, pair_address: Addr) -> Decimal256 {
    let book_query_msg = FINQueryMsg::Book {
        limit: Some(1),
        offset: None,
    };

    let book_response: FINBookResponse = querier
        .query_wasm_smart(pair_address, &book_query_msg)
        .unwrap();

    book_response.base[0].quote_price
}

pub fn query_quote_price(querier: QuerierWrapper, pair_address: Addr) -> Decimal256 {
    let book_query_msg = FINQueryMsg::Book {
        limit: Some(1),
        offset: None,
    };

    let book_response: FINBookResponse = querier
        .query_wasm_smart(pair_address, &book_query_msg)
        .unwrap();

    book_response.quote[0].quote_price
}

pub fn query_order_details(
    querier: QuerierWrapper,
    pair_address: Addr,
    order_idx: Uint128,
) -> StdResult<(Uint128, Uint128, Uint128)> {
    let fin_order_query_msg = FINQueryMsg::Order { order_idx };

    let order_response: FINOrderResponseWithoutDenom =
        querier.query_wasm_smart(pair_address, &fin_order_query_msg)?;

    Ok((
        order_response.offer_amount,
        order_response.original_offer_amount,
        order_response.filled_amount,
    ))
}
