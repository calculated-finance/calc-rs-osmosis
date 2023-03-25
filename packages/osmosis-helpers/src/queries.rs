use crate::{msg::FinConfigResponse, position_type::PositionType};
use base::{pool::Pool, price_type::PriceType};
use cosmwasm_std::{Coin, Decimal, QuerierWrapper, StdError, StdResult, Uint128};
use osmosis_std::types::osmosis::gamm::v2::QuerySpotPriceRequest;
fn _query_quote_price(
    _querier: QuerierWrapper,
    _pool: &Pool,
    _swap_denom: &str,
) -> StdResult<Decimal> {
    unimplemented!()
}

pub fn query_belief_price(
    _querier: QuerierWrapper,
    _pool: &Pool,
    _swap_denom: &str,
) -> StdResult<Decimal> {
    unimplemented!()
}

pub fn query_price(
    querier: QuerierWrapper,
    pool: Pool,
    swap_amount: &Coin,
    _price_type: PriceType,
) -> StdResult<Decimal> {
    if ![pool.base_denom.clone(), pool.quote_denom.clone()].contains(&swap_amount.denom) {
        return Err(StdError::generic_err(format!(
            "Provided swap denom {} not in pool {}",
            swap_amount.denom, pool.pool_id
        )));
    }

    let position_type = match swap_amount.denom == pool.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let base_asset_denom;
    let quote_asset_denom;

    match position_type {
        PositionType::Enter => {
            base_asset_denom = pool.base_denom;
            quote_asset_denom = pool.quote_denom;
        }
        PositionType::Exit => {
            base_asset_denom = pool.quote_denom;
            quote_asset_denom = pool.base_denom;
        }
    }

    QuerySpotPriceRequest {
        pool_id: pool.pool_id,
        base_asset_denom,
        quote_asset_denom,
    }
    .query(&querier)
    .unwrap()
    .spot_price
    .parse::<Decimal>()
}

pub fn calculate_slippage(actual_price: Decimal, belief_price: Decimal) -> Decimal {
    let difference = actual_price
        .checked_sub(belief_price)
        .unwrap_or(Decimal::zero());

    if difference.is_zero() {
        return Decimal::zero();
    }

    difference / belief_price
}

pub fn query_pool_config(_querier: QuerierWrapper, _pool_id: u64) -> StdResult<FinConfigResponse> {
    unimplemented!()
}

// #[cfg(test)]
// mod query_quote_price_tests {
//     use crate::{
//         constants::{ONE, ONE_DECIMAL, TEN_MICRONS},
//         msg::FinBookResponse,
//         queries::query_quote_price,
//         test_helpers::set_fin_price,
//     };
//     use base::pool::Pool;
//     use cosmwasm_std::{
//         from_binary, testing::mock_dependencies, to_binary, Addr, QueryRequest, WasmQuery,
//     };
//     use kujira::fin::QueryMsg;

//     #[test]
//     fn quote_price_comes_from_quote_book_for_fin_sell() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let book = from_binary::<FinBookResponse>(
//             &deps
//                 .querier
//                 .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
//                     contract_addr: 0.to_string(),
//                     msg: to_binary(&QueryMsg::Book {
//                         limit: Some(20),
//                         offset: Some(0),
//                     })
//                     .unwrap(),
//                 }))
//                 .unwrap()
//                 .unwrap(),
//         )
//         .unwrap();

//         let response = query_quote_price(
//             deps.as_ref().querier,
//             &Pool {
//                 pool_id: 0,
//                 base_denom: "base".to_string(),
//                 quote_denom: "quote".to_string(),
//             },
//             "base",
//         )
//         .unwrap();

//         assert_eq!(book.quote.first().unwrap().quote_price, response);
//     }

//     #[test]
//     fn quote_price_comes_from_base_book_for_fin_buy() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let book = from_binary::<FinBookResponse>(
//             &deps
//                 .querier
//                 .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
//                     contract_addr: 0.to_string(),
//                     msg: to_binary(&QueryMsg::Book {
//                         limit: Some(20),
//                         offset: Some(0),
//                     })
//                     .unwrap(),
//                 }))
//                 .unwrap()
//                 .unwrap(),
//         )
//         .unwrap();

//         let response = query_quote_price(
//             deps.as_ref().querier,
//             &Pool {
//                 pool_id: 0,
//                 base_denom: "base".to_string(),
//                 quote_denom: "quote".to_string(),
//             },
//             "quote",
//         )
//         .unwrap();

//         assert_eq!(book.base.first().unwrap().quote_price, response);
//     }
// }

// #[cfg(test)]
// mod query_belief_price_tests {
//     use crate::{
//         constants::{ONE, ONE_DECIMAL, TEN_MICRONS},
//         queries::{query_belief_price, query_quote_price},
//         test_helpers::set_fin_price,
//     };
//     use base::pool::Pool;
//     use cosmwasm_std::{testing::mock_dependencies, Addr, Decimal};

//     #[test]
//     fn belief_price_is_quote_price_for_fin_buy() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let pair = &Pool {
//             pool_id: 0,
//             base_denom: "base".to_string(),
//             quote_denom: "quote".to_string(),
//         };

//         let quote_price = query_quote_price(deps.as_ref().querier, pair, "quote").unwrap();

//         let belief_price = query_belief_price(
//             deps.as_ref().querier,
//             &Pool {
//                 pool_id: 0,
//                 base_denom: "base".to_string(),
//                 quote_denom: "quote".to_string(),
//             },
//             "quote",
//         )
//         .unwrap();

//         assert_eq!(quote_price, belief_price);
//     }

//     #[test]
//     fn belief_price_is_inverted_quote_price_for_fin_sell() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let pair = &Pool {
//             pool_id: 0,
//             base_denom: "base".to_string(),
//             quote_denom: "quote".to_string(),
//         };

//         let quote_price = query_quote_price(deps.as_ref().querier, pair, "base").unwrap();
//         let belief_price = query_belief_price(deps.as_ref().querier, pair, "base").unwrap();

//         assert_eq!(quote_price, Decimal::one() / belief_price);
//     }
// }

// #[cfg(test)]
// mod query_actual_price_tests {
//     use crate::{
//         constants::{ONE, ONE_DECIMAL, TEN, TEN_MICRONS},
//         queries::{query_belief_price, query_price},
//         test_helpers::set_fin_price,
//     };
//     use base::{pool::Pool, price_type::PriceType};
//     use cosmwasm_std::{testing::mock_dependencies, Addr, Coin};

//     #[test]
//     fn actual_price_equals_belief_price_when_swap_amount_is_small() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let pair = Pool {
//             pool_id: 0,
//             base_denom: "base".to_string(),
//             quote_denom: "quote".to_string(),
//         };

//         let belief_price = query_belief_price(deps.as_ref().querier, &pair, "base").unwrap();

//         let actual_price = query_price(
//             deps.as_ref().querier,
//             pair,
//             &Coin::new(100, "base"),
//             PriceType::Actual,
//         )
//         .unwrap();

//         assert_eq!(belief_price, actual_price);
//     }

//     #[test]
//     fn actual_price_higher_than_belief_price_when_swap_amount_is_large_for_fin_buy() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let pair = Pool {
//             pool_id: 0,
//             base_denom: "base".to_string(),
//             quote_denom: "quote".to_string(),
//         };

//         let swap_denom = "base";

//         let belief_price = query_belief_price(deps.as_ref().querier, &pair, swap_denom).unwrap();

//         let actual_price = query_price(
//             deps.as_ref().querier,
//             pair,
//             &Coin::new((ONE + ONE).into(), swap_denom),
//             PriceType::Actual,
//         )
//         .unwrap();

//         assert!(actual_price > belief_price);
//     }

//     #[test]
//     fn actual_price_higher_than_belief_price_when_swap_amount_is_large_for_fin_sell() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let pair = Pool {
//             pool_id: 0,
//             base_denom: "base".to_string(),
//             quote_denom: "quote".to_string(),
//         };

//         let swap_denom = "quote";

//         let belief_price = query_belief_price(deps.as_ref().querier, &pair, swap_denom).unwrap();

//         let actual_price = query_price(
//             deps.as_ref().querier,
//             pair,
//             &Coin::new((ONE + ONE).into(), swap_denom),
//             PriceType::Actual,
//         )
//         .unwrap();

//         assert!(actual_price > belief_price);
//     }

//     #[test]
//     fn throws_error_when_book_depth_is_small_than_swap_amount() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let pair = Pool {
//             pool_id: 0,
//             base_denom: "base".to_string(),
//             quote_denom: "quote".to_string(),
//         };

//         let swap_denom = "quote";

//         let error = query_price(
//             deps.as_ref().querier,
//             pair,
//             &Coin::new((TEN + TEN).into(), swap_denom),
//             PriceType::Actual,
//         )
//         .unwrap_err();

//         assert_eq!(
//             error.to_string(),
//             "Generic error: Not enough liquidity to swap 20000000quote"
//         );
//     }

//     #[test]
//     fn throws_error_when_swap_denom_not_in_pair() {
//         let mut deps = mock_dependencies();

//         set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

//         let pair = Pool {
//             pool_id: 0,
//             base_denom: "base".to_string(),
//             quote_denom: "quote".to_string(),
//         };

//         let swap_denom = "other";

//         let error = query_price(
//             deps.as_ref().querier,
//             pair.clone(),
//             &Coin::new(TEN.into(), swap_denom),
//             PriceType::Actual,
//         )
//         .unwrap_err();

//         assert_eq!(
//             error.to_string(),
//             format!(
//                 "Generic error: Provided swap denom {} not in pair {}",
//                 swap_denom, pair.pool_id
//             )
//         );
//     }
// }
