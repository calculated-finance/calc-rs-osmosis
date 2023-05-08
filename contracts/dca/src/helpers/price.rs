use super::routes::{calculate_route, get_pool, get_token_out_denom};
use crate::{
    state::config::get_config,
    types::{pair::Pair, position_type::PositionType},
};
use cosmwasm_std::{Coin, Decimal, Deps, Env, QuerierWrapper, StdResult, Uint128};
use osmosis_std::{
    shim::Timestamp,
    types::osmosis::{
        poolmanager::v1beta1::PoolmanagerQuerier, twap::v1beta1::ArithmeticTwapRequest,
    },
};

pub fn query_belief_price(
    deps: &Deps,
    env: &Env,
    pair: &Pair,
    mut swap_denom: String,
) -> StdResult<Decimal> {
    let pool_ids = match pair.position_type(swap_denom.clone()) {
        PositionType::Enter => pair.route.clone(),
        PositionType::Exit => pair.route.clone().into_iter().rev().collect(),
    };

    let mut price = Decimal::one();

    let config = get_config(deps.storage)?;

    for pool_id in pool_ids.into_iter() {
        let target_denom = get_token_out_denom(&deps.querier, swap_denom.clone(), pool_id)?;

        let pool = get_pool(&deps.querier, pool_id)?;

        let swap_fee = pool
            .pool_params
            .unwrap()
            .swap_fee
            .parse::<Decimal>()
            .unwrap();

        let pool_price = ArithmeticTwapRequest {
            pool_id,
            base_asset: target_denom.clone(),
            quote_asset: swap_denom,
            start_time: Some(Timestamp {
                seconds: (env.block.time.seconds() - config.twap_period) as i64,
                nanos: env.block.time.nanos() as i32,
            }),
            end_time: None,
        }
        .query(&deps.querier)?
        .arithmetic_twap
        .parse::<Decimal>()?
            * (Decimal::one() + swap_fee);

        price = pool_price * price;

        swap_denom = target_denom;
    }

    Ok(price)
}

pub fn query_price(
    querier: &QuerierWrapper,
    env: &Env,
    pair: &Pair,
    swap_amount: &Coin,
) -> StdResult<Decimal> {
    let routes = calculate_route(querier, pair, swap_amount.denom.clone())?;

    let token_out_amount = PoolmanagerQuerier::new(querier)
        .estimate_swap_exact_amount_in(
            env.contract.address.to_string(),
            0,
            swap_amount.to_string(),
            routes.clone(),
        )
        .unwrap_or_else(|_| {
            panic!(
                "amount of {} received for swapping {} via {:#?}",
                routes.last().unwrap().token_out_denom,
                swap_amount,
                routes
            )
        })
        .token_out_amount
        .parse::<Uint128>()?;

    Ok(Decimal::from_ratio(swap_amount.amount, token_out_amount))
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

#[cfg(test)]
mod query_belief_price_tests {
    use std::str::FromStr;

    use super::*;
    use crate::{
        constants::SWAP_FEE_RATE,
        tests::{
            helpers::instantiate_contract,
            mocks::{calc_mock_dependencies, ADMIN},
        },
    };
    use cosmwasm_std::{
        testing::{mock_env, mock_info},
        to_binary, StdError,
    };
    use osmosis_std::types::osmosis::twap::v1beta1::ArithmeticTwapResponse;
    use prost::Message;

    #[test]
    fn query_belief_price_with_single_pool_id_should_succeed() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        deps.querier.update_stargate(|path, data| {
            if path == "/osmosis.twap.v1beta1.Query/ArithmeticTwap" {
                let price = match ArithmeticTwapRequest::decode(data.as_slice())
                    .unwrap()
                    .pool_id
                {
                    3 => "0.8",
                    _ => "1.0",
                };

                return to_binary(&ArithmeticTwapResponse {
                    arithmetic_twap: price.to_string(),
                });
            }
            Err(StdError::generic_err("invoke fallback"))
        });

        let pair = Pair::default();

        let price =
            query_belief_price(&deps.as_ref(), &env, &pair.clone(), pair.quote_denom).unwrap();

        assert_eq!(
            price,
            Decimal::percent(80) * (Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap())
        );
    }

    #[test]
    fn query_belief_price_with_multiple_pool_ids_id_should_succeed() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        deps.querier.update_stargate(|path, data| {
            if path == "/osmosis.twap.v1beta1.Query/ArithmeticTwap" {
                let price = match ArithmeticTwapRequest::decode(data.as_slice())
                    .unwrap()
                    .pool_id
                {
                    1 => "0.2",
                    4 => "1.2",
                    _ => "1.0",
                };

                return to_binary(&ArithmeticTwapResponse {
                    arithmetic_twap: price.to_string(),
                });
            }
            Err(StdError::generic_err("invoke fallback"))
        });

        let pair = Pair {
            route: vec![4, 1],
            ..Pair::default()
        };

        let price =
            query_belief_price(&deps.as_ref(), &env, &pair.clone(), pair.quote_denom).unwrap();

        assert_eq!(
            price,
            Decimal::percent(20)
                * (Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap())
                * Decimal::percent(120)
                * (Decimal::one() + Decimal::from_str(SWAP_FEE_RATE).unwrap())
        );
    }
}
