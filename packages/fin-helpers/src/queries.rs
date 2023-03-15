use crate::{
    msg::{FinBookResponse, FinConfigResponse, FinOrderResponseWithoutDenom},
    position_type::PositionType,
};
use base::{pair::Pair, price_type::PriceType};
use cosmwasm_std::{Addr, Coin, Decimal, Decimal256, QuerierWrapper, StdError, StdResult, Uint128};
use kujira::fin::QueryMsg as FinQueryMsg;

pub fn query_base_price(querier: QuerierWrapper, pair_address: Addr) -> Decimal256 {
    let book_query_msg = FinQueryMsg::Book {
        limit: Some(1),
        offset: None,
    };

    let book_response: FinBookResponse = querier
        .query_wasm_smart(pair_address, &book_query_msg)
        .unwrap();

    book_response.base[0].quote_price.into()
}

pub fn query_quote_price(querier: QuerierWrapper, pair_address: Addr) -> Decimal256 {
    let book_query_msg = FinQueryMsg::Book {
        limit: Some(1),
        offset: None,
    };

    let book_response: FinBookResponse = querier
        .query_wasm_smart(pair_address, &book_query_msg)
        .unwrap();

    book_response.quote[0].quote_price.into()
}

pub fn query_belief_price(
    querier: QuerierWrapper,
    pair: Pair,
    swap_denom: &str,
) -> StdResult<Decimal> {
    let position_type = match swap_denom == pair.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let book_response = querier.query_wasm_smart::<FinBookResponse>(
        pair.address,
        &FinQueryMsg::Book {
            limit: Some(1),
            offset: None,
        },
    )?;

    let book_price = match position_type {
        PositionType::Enter => book_response.base,
        PositionType::Exit => book_response.quote,
    }[0]
    .quote_price;

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
            "Provided swap denom {} not in pair {:?}",
            swap_amount, pair
        )));
    }

    if price_type == PriceType::Belief {
        return query_belief_price(querier, pair, &swap_amount.denom);
    }

    let position_type = match swap_amount.denom == pair.quote_denom {
        true => PositionType::Enter,
        false => PositionType::Exit,
    };

    let mut spent = Uint128::zero();
    let mut received = Uint128::zero();
    let mut limit = 20;
    let mut offset = Some(0);

    while spent <= swap_amount.amount {
        let book_response = querier.query_wasm_smart::<FinBookResponse>(
            pair.address.clone(),
            &FinQueryMsg::Book {
                limit: Some(limit),
                offset,
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

            if cost_in_swap_denom < swap_amount_remaining {
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

        if Decimal::from_ratio(spent, swap_amount.amount) < Decimal::percent(50) {
            limit = limit * 2;
        }

        offset = offset.map(|o| o + limit);
    }

    if spent < swap_amount.amount || received.is_zero() {
        return Err(StdError::generic_err(format!(
            "Not enough liquidity to swap {}",
            swap_amount
        )));
    }

    Ok(Decimal::from_ratio(swap_amount.amount, received))
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
