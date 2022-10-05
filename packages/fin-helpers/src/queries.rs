use cosmwasm_std::{Addr, Decimal256, QuerierWrapper, Uint128};
use kujira::fin::{BookResponse, QueryMsg as FINQueryMsg};
use serde::Deserialize;

pub fn query_base_price(querier: QuerierWrapper, pair_address: Addr) -> Decimal256 {
    let book_query_msg = FINQueryMsg::Book {
        limit: Some(1),
        offset: None,
    };

    let book_response: BookResponse = querier
        .query_wasm_smart(pair_address, &book_query_msg)
        .unwrap();

    book_response.base[0].quote_price
}

pub fn query_quote_price(querier: QuerierWrapper, pair_address: Addr) -> Decimal256 {
    let book_query_msg = FINQueryMsg::Book {
        limit: Some(1),
        offset: None,
    };

    let book_response: BookResponse = querier
        .query_wasm_smart(pair_address, &book_query_msg)
        .unwrap();

    book_response.quote[0].quote_price
}

pub fn query_order_details(
    querier: QuerierWrapper,
    pair_address: Addr,
    order_idx: Uint128,
) -> (Uint128, Uint128, Uint128) {
    let fin_order_query_msg = FINQueryMsg::Order { order_idx };

    #[derive(Deserialize)]
    struct CalcOrderResponse {
        pub offer_amount: Uint128,
        pub filled_amount: Uint128,
        pub original_offer_amount: Uint128,
    }

    let order_response: CalcOrderResponse = querier
        .query_wasm_smart(pair_address, &fin_order_query_msg)
        .unwrap();

    (
        order_response.offer_amount,
        order_response.original_offer_amount,
        order_response.filled_amount,
    )
}
