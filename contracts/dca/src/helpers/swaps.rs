use super::routes::calculate_route;
use crate::types::pair::Pair;
use cosmwasm_std::{Coin, Decimal, Env, QuerierWrapper, ReplyOn, StdResult, SubMsg, Uint128};
use osmosis_std::types::osmosis::poolmanager::v1beta1::MsgSwapExactAmountIn;
use std::cmp::max;

pub fn create_osmosis_swap_message(
    querier: &QuerierWrapper,
    env: &Env,
    pair: &Pair,
    swap_amount: Coin,
    slippage_tolerance: Decimal,
    belief_price: Decimal,
    minimum_receive_amount: Option<Uint128>,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let routes = calculate_route(querier, pair, swap_amount.denom.clone())?;

    let expected_receive_amount = swap_amount.amount
        * (Decimal::one() / belief_price)
        * (Decimal::one() - slippage_tolerance);

    let token_out_min_amount = minimum_receive_amount
        .map_or(expected_receive_amount, |minimum_receive_amount| {
            max(minimum_receive_amount, expected_receive_amount)
        });

    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: MsgSwapExactAmountIn {
            sender: env.contract.address.to_string(),
            token_in: Some(swap_amount.into()),
            token_out_min_amount: token_out_min_amount.to_string(),
            routes,
        }
        .into(),
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}

#[cfg(test)]
mod create_osmosis_swap_message_tests {
    use super::create_osmosis_swap_message;
    use crate::{
        constants::{ONE, TWO_MICRONS},
        helpers::routes::calculate_route,
        tests::mocks::{calc_mock_dependencies, DENOM_UOSMO},
        types::pair::Pair,
    };
    use cosmwasm_std::{testing::mock_env, Coin, Decimal, ReplyOn, SubMsg};
    use osmosis_std::types::osmosis::poolmanager::v1beta1::MsgSwapExactAmountIn;

    #[test]
    fn uses_minimum_receive_amount_if_larger_than_expected_receive_amount() {
        let deps = calc_mock_dependencies();
        let env = mock_env();

        let swap_amount = Coin::new(ONE.into(), DENOM_UOSMO);
        let minimum_receive_amount = Some(ONE);
        let belief_price = Decimal::one();
        let pair = Pair::default();
        let slippage_tolerance = Decimal::percent(100);

        let msg = create_osmosis_swap_message(
            &deps.as_ref().querier,
            &env,
            &pair,
            swap_amount.clone(),
            slippage_tolerance,
            belief_price,
            minimum_receive_amount,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            msg,
            SubMsg {
                id: 0,
                msg: MsgSwapExactAmountIn {
                    sender: env.contract.address.to_string(),
                    token_in: Some(swap_amount.clone().into()),
                    token_out_min_amount: minimum_receive_amount.unwrap().to_string(),
                    routes: calculate_route(&deps.as_ref().querier, &pair, swap_amount.denom)
                        .unwrap(),
                }
                .into(),
                gas_limit: None,
                reply_on: ReplyOn::Never,
            }
        )
    }

    #[test]
    fn uses_expected_receive_amount_if_larger_than_minimum_receive_amount() {
        let deps = calc_mock_dependencies();
        let env = mock_env();

        let swap_amount = Coin::new(ONE.into(), DENOM_UOSMO);
        let minimum_receive_amount = Some(ONE / TWO_MICRONS);
        let belief_price = Decimal::one();
        let pair = Pair::default();
        let slippage_tolerance = Decimal::percent(1);

        let msg = create_osmosis_swap_message(
            &deps.as_ref().querier,
            &env,
            &pair,
            swap_amount.clone(),
            slippage_tolerance,
            belief_price,
            minimum_receive_amount,
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            msg,
            SubMsg {
                id: 0,
                msg: MsgSwapExactAmountIn {
                    sender: env.contract.address.to_string(),
                    token_in: Some(swap_amount.clone().into()),
                    token_out_min_amount: (swap_amount.amount
                        * (Decimal::one() - slippage_tolerance))
                        .to_string(),
                    routes: calculate_route(&deps.as_ref().querier, &pair, swap_amount.denom)
                        .unwrap(),
                }
                .into(),
                gas_limit: None,
                reply_on: ReplyOn::Never,
            }
        )
    }
}
