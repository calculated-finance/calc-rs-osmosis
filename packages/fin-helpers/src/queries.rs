use cosmwasm_std::{Addr, Decimal256, QuerierWrapper, StdResult, Uint128};
use kujira::fin::QueryMsg as FINQueryMsg;

use crate::msg::{FINBookResponse, FINConfigResponse, FINOrderResponseWithoutDenom};

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
) -> StdResult<FINOrderResponseWithoutDenom> {
    let fin_order_query_msg = FINQueryMsg::Order { order_idx };
    Ok(querier.query_wasm_smart(pair_address, &fin_order_query_msg)?)
}

pub fn query_pair_config(
    querier: QuerierWrapper,
    pair_address: Addr,
) -> StdResult<FINConfigResponse> {
    let fin_pair_config_query_msg = FINQueryMsg::Config {};

    let pair_config_response: FINConfigResponse =
        querier.query_wasm_smart(pair_address, &fin_pair_config_query_msg)?;

    Ok(pair_config_response)
}
