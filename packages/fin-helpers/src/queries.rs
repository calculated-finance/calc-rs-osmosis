use crate::{
    msg::{FinBookResponse, FinConfigResponse, FinOrderResponseWithoutDenom},
    position_type::PositionType,
};
use base::{pair::Pair, price_type::PriceType};
use cosmwasm_std::{Addr, Coin, Decimal, QuerierWrapper, StdError, StdResult, Uint128};
use kujira::fin::QueryMsg as FinQueryMsg;

fn query_quote_price(querier: QuerierWrapper, pair: &Pair, swap_denom: &str) -> StdResult<Decimal> {
    let position_type = match swap_denom == pair.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let book_response = querier.query_wasm_smart::<FinBookResponse>(
        pair.address.clone(),
        &FinQueryMsg::Book {
            limit: Some(1),
            offset: None,
        },
    )?;

    let book = match position_type {
        PositionType::Enter => book_response.base,
        PositionType::Exit => book_response.quote,
    };

    if book.is_empty() {
        return Err(StdError::generic_err(format!(
            "No orders found for pair {:?}",
            pair
        )));
    }

    Ok(book[0].quote_price)
}

pub fn query_belief_price(
    querier: QuerierWrapper,
    pair: &Pair,
    swap_denom: &str,
) -> StdResult<Decimal> {
    let position_type = match swap_denom == pair.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let book_price = query_quote_price(querier, &pair, swap_denom)?;

    Ok(match position_type {
        PositionType::Enter => book_price,
        PositionType::Exit => Decimal::one()
            .checked_div(book_price)
            .expect("should return a valid inverted price for fin sell"),
    })
}

pub fn query_price(
    querier: QuerierWrapper,
    pair: Pair,
    swap_amount: &Coin,
    price_type: PriceType,
) -> StdResult<Decimal> {
    if ![pair.base_denom.clone(), pair.quote_denom.clone()].contains(&swap_amount.denom) {
        return Err(StdError::generic_err(format!(
            "Provided swap denom {} not in pair {}",
            swap_amount.denom, pair.address
        )));
    }

    if price_type == PriceType::Belief || swap_amount.amount == Uint128::zero() {
        return query_belief_price(querier, &pair, &swap_amount.denom);
    }

    let position_type = match swap_amount.denom == pair.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let mut spent = Uint128::zero();
    let mut received = Uint128::zero();
    let mut limit = 20;
    let mut offset = 0;

    while spent <= swap_amount.amount {
        let book_response = querier.query_wasm_smart::<FinBookResponse>(
            pair.address.clone(),
            &FinQueryMsg::Book {
                limit: Some(limit),
                offset: Some(offset),
            },
        )?;

        let book = match position_type {
            PositionType::Enter => book_response.base,
            PositionType::Exit => book_response.quote,
        };

        if book.is_empty() {
            break;
        }

        book.iter().for_each(|order| {
            let price_in_swap_denom = match position_type {
                PositionType::Enter => order.quote_price,
                PositionType::Exit => Decimal::one()
                    .checked_div(order.quote_price)
                    .expect("order price in swap denom"),
            };

            let cost_in_swap_denom = order.total_offer_amount * price_in_swap_denom;
            let swap_amount_remaining = swap_amount.amount - spent;

            if cost_in_swap_denom == Uint128::zero() {
                received += order.total_offer_amount;
            } else if cost_in_swap_denom < swap_amount_remaining {
                spent += cost_in_swap_denom;
                received += order.total_offer_amount;
            } else {
                spent = swap_amount.amount;
                let portion_of_order_to_fill =
                    Decimal::from_ratio(swap_amount_remaining, cost_in_swap_denom);
                received += order.total_offer_amount * portion_of_order_to_fill;
                return;
            }
        });

        if spent == swap_amount.amount {
            break;
        }

        offset = offset.checked_add(limit).unwrap_or(u8::MAX);

        if Decimal::from_ratio(spent, swap_amount.amount) < Decimal::percent(50) {
            limit = limit.checked_mul(2).unwrap_or(u8::MAX);
        }
    }

    if spent < swap_amount.amount || received.is_zero() {
        return Err(StdError::generic_err(format!(
            "Not enough liquidity to swap {}",
            swap_amount
        )));
    }

    Ok(Decimal::from_ratio(swap_amount.amount, received))
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

pub fn query_order_details(
    querier: QuerierWrapper,
    pair_address: Addr,
    order_idx: Uint128,
) -> StdResult<FinOrderResponseWithoutDenom> {
    let fin_order_query_msg = FinQueryMsg::Order { order_idx };
    Ok(querier.query_wasm_smart(pair_address, &fin_order_query_msg)?)
}

pub fn query_pair_config(
    querier: QuerierWrapper,
    pair_address: Addr,
) -> StdResult<FinConfigResponse> {
    let fin_pair_config_query_msg = FinQueryMsg::Config {};

    let pair_config_response: FinConfigResponse =
        querier.query_wasm_smart(pair_address, &fin_pair_config_query_msg)?;

    Ok(pair_config_response)
}

#[cfg(test)]
mod query_quote_price_tests {
    use crate::{
        constants::{ONE, ONE_DECIMAL, TEN_MICRONS},
        msg::FinBookResponse,
        queries::query_quote_price,
        test_helpers::set_fin_price,
    };
    use base::pair::Pair;
    use cosmwasm_std::{
        from_binary, testing::mock_dependencies, to_binary, Addr, QueryRequest, WasmQuery,
    };
    use kujira::fin::QueryMsg;

    #[test]
    fn quote_price_comes_from_quote_book_for_fin_sell() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let book = from_binary::<FinBookResponse>(
            &deps
                .querier
                .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: Addr::unchecked("pair").to_string(),
                    msg: to_binary(&QueryMsg::Book {
                        limit: Some(20),
                        offset: Some(0),
                    })
                    .unwrap(),
                }))
                .unwrap()
                .unwrap(),
        )
        .unwrap();

        let response = query_quote_price(
            deps.as_ref().querier,
            &Pair {
                address: Addr::unchecked("pair"),
                base_denom: "base".to_string(),
                quote_denom: "quote".to_string(),
            },
            "base",
        )
        .unwrap();

        assert_eq!(book.quote.first().unwrap().quote_price, response);
    }

    #[test]
    fn quote_price_comes_from_base_book_for_fin_buy() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let book = from_binary::<FinBookResponse>(
            &deps
                .querier
                .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: Addr::unchecked("pair").to_string(),
                    msg: to_binary(&QueryMsg::Book {
                        limit: Some(20),
                        offset: Some(0),
                    })
                    .unwrap(),
                }))
                .unwrap()
                .unwrap(),
        )
        .unwrap();

        let response = query_quote_price(
            deps.as_ref().querier,
            &Pair {
                address: Addr::unchecked("pair"),
                base_denom: "base".to_string(),
                quote_denom: "quote".to_string(),
            },
            "quote",
        )
        .unwrap();

        assert_eq!(book.base.first().unwrap().quote_price, response);
    }
}

#[cfg(test)]
mod query_belief_price_tests {
    use crate::{
        constants::{ONE, ONE_DECIMAL, TEN_MICRONS},
        queries::{query_belief_price, query_quote_price},
        test_helpers::set_fin_price,
    };
    use base::pair::Pair;
    use cosmwasm_std::{testing::mock_dependencies, Addr, Decimal};

    #[test]
    fn belief_price_is_quote_price_for_fin_buy() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let pair = &Pair {
            address: Addr::unchecked("pair"),
            base_denom: "base".to_string(),
            quote_denom: "quote".to_string(),
        };

        let quote_price = query_quote_price(deps.as_ref().querier, pair, "quote").unwrap();

        let belief_price = query_belief_price(
            deps.as_ref().querier,
            &Pair {
                address: Addr::unchecked("pair"),
                base_denom: "base".to_string(),
                quote_denom: "quote".to_string(),
            },
            "quote",
        )
        .unwrap();

        assert_eq!(quote_price, belief_price);
    }

    #[test]
    fn belief_price_is_inverted_quote_price_for_fin_sell() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let pair = &Pair {
            address: Addr::unchecked("pair"),
            base_denom: "base".to_string(),
            quote_denom: "quote".to_string(),
        };

        let quote_price = query_quote_price(deps.as_ref().querier, pair, "base").unwrap();
        let belief_price = query_belief_price(deps.as_ref().querier, pair, "base").unwrap();

        assert_eq!(quote_price, Decimal::one() / belief_price);
    }
}

#[cfg(test)]
mod query_actual_price_tests {
    use crate::{
        constants::{ONE, ONE_DECIMAL, TEN, TEN_MICRONS},
        queries::{query_belief_price, query_price},
        test_helpers::set_fin_price,
    };
    use base::{pair::Pair, price_type::PriceType};
    use cosmwasm_std::{testing::mock_dependencies, Addr, Coin};

    #[test]
    fn actual_price_equals_belief_price_when_swap_amount_is_small() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "base".to_string(),
            quote_denom: "quote".to_string(),
        };

        let belief_price = query_belief_price(deps.as_ref().querier, &pair, "base").unwrap();

        let actual_price = query_price(
            deps.as_ref().querier,
            pair,
            &Coin::new(100, "base"),
            PriceType::Actual,
        )
        .unwrap();

        assert_eq!(belief_price, actual_price);
    }

    #[test]
    fn actual_price_higher_than_belief_price_when_swap_amount_is_large_for_fin_buy() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "base".to_string(),
            quote_denom: "quote".to_string(),
        };

        let swap_denom = "base";

        let belief_price = query_belief_price(deps.as_ref().querier, &pair, swap_denom).unwrap();

        let actual_price = query_price(
            deps.as_ref().querier,
            pair,
            &Coin::new((ONE + ONE).into(), swap_denom),
            PriceType::Actual,
        )
        .unwrap();

        assert!(actual_price > belief_price);
    }

    #[test]
    fn actual_price_higher_than_belief_price_when_swap_amount_is_large_for_fin_sell() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "base".to_string(),
            quote_denom: "quote".to_string(),
        };

        let swap_denom = "quote";

        let belief_price = query_belief_price(deps.as_ref().querier, &pair, swap_denom).unwrap();

        let actual_price = query_price(
            deps.as_ref().querier,
            pair,
            &Coin::new((ONE + ONE).into(), swap_denom),
            PriceType::Actual,
        )
        .unwrap();

        assert!(actual_price > belief_price);
    }

    #[test]
    fn throws_error_when_book_depth_is_small_than_swap_amount() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "base".to_string(),
            quote_denom: "quote".to_string(),
        };

        let swap_denom = "quote";

        let error = query_price(
            deps.as_ref().querier,
            pair,
            &Coin::new((TEN + TEN).into(), swap_denom),
            PriceType::Actual,
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Generic error: Not enough liquidity to swap 20000000quote"
        );
    }

    #[test]
    fn throws_error_when_swap_denom_not_in_pair() {
        let mut deps = mock_dependencies();

        set_fin_price(&mut deps, &ONE_DECIMAL, &ONE, &TEN_MICRONS);

        let pair = Pair {
            address: Addr::unchecked("pair"),
            base_denom: "base".to_string(),
            quote_denom: "quote".to_string(),
        };

        let swap_denom = "other";

        let error = query_price(
            deps.as_ref().querier,
            pair.clone(),
            &Coin::new(TEN.into(), swap_denom),
            PriceType::Actual,
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            format!(
                "Generic error: Provided swap denom {} not in pair {}",
                swap_denom, pair.address
            )
        );
    }
}
