use crate::types::{pair::Pair, position_type::PositionType};
use cosmwasm_std::{QuerierWrapper, StdError, StdResult};
use osmosis_std::types::osmosis::{
    gamm::v1beta1::{GammQuerier, Pool},
    poolmanager::v1beta1::SwapAmountInRoute,
};
use prost::DecodeError;

pub fn get_token_out_denom(
    querier: &QuerierWrapper,
    token_in_denom: String,
    pool_id: u64,
) -> StdResult<String> {
    let pool: Pool = GammQuerier::new(querier)
        .pool(pool_id)?
        .pool
        .expect(&format!("pool id {}", pool_id))
        .try_into()
        .map_err(|e: DecodeError| StdError::ParseErr {
            target_type: Pool::TYPE_URL.to_string(),
            msg: e.to_string(),
        })?;

    let token_out_denom = pool
        .pool_assets
        .iter()
        .find(|asset| asset.token.clone().unwrap().denom != token_in_denom)
        .map(|asset| asset.token.clone().unwrap().denom.clone())
        .ok_or(StdError::generic_err("no token out denom found"));

    token_out_denom
}

pub fn calculate_route(
    querier: &QuerierWrapper,
    pair: &Pair,
    mut swap_denom: String,
) -> StdResult<Vec<SwapAmountInRoute>> {
    let pool_ids = match pair.position_type(swap_denom.clone()) {
        PositionType::Enter => pair.route.clone(),
        PositionType::Exit => pair.route.clone().into_iter().rev().collect(),
    };

    let mut route: Vec<SwapAmountInRoute> = vec![];

    for pool_id in pool_ids.into_iter() {
        let target_denom = get_token_out_denom(querier, swap_denom, pool_id)?;

        route.push(SwapAmountInRoute {
            pool_id,
            token_out_denom: target_denom.clone(),
        });

        swap_denom = target_denom;
    }

    Ok(route)
}

#[cfg(test)]
mod calculate_route_tests {
    use super::calculate_route;
    use crate::{
        tests::mocks::{calc_mock_dependencies, DENOM_UATOM, DENOM_UION, DENOM_UOSMO, DENOM_USDC},
        types::pair::Pair,
    };
    use osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute;

    #[test]
    fn calculates_1_pool_route() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![1],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UOSMO.to_string(),
            ..Pair::default()
        };

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UATOM.to_string()).unwrap(),
            vec![SwapAmountInRoute {
                pool_id: 1,
                token_out_denom: DENOM_UOSMO.to_string(),
            }]
        );

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UOSMO.to_string()).unwrap(),
            vec![SwapAmountInRoute {
                pool_id: 1,
                token_out_denom: DENOM_UATOM.to_string(),
            }]
        );
    }

    #[test]
    fn calculates_2_pool_route() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![1, 2],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UION.to_string(),
            ..Pair::default()
        };

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UATOM.to_string()).unwrap(),
            vec![
                SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: DENOM_UOSMO.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 2,
                    token_out_denom: DENOM_UION.to_string(),
                }
            ]
        );

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UION.to_string()).unwrap(),
            vec![
                SwapAmountInRoute {
                    pool_id: 2,
                    token_out_denom: DENOM_UOSMO.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: DENOM_UATOM.to_string(),
                }
            ]
        );
    }

    #[test]
    fn calculates_3_pool_route() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![3, 2, 1],
            quote_denom: DENOM_USDC.to_string(),
            base_denom: DENOM_UATOM.to_string(),
            ..Pair::default()
        };

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_USDC.to_string()).unwrap(),
            vec![
                SwapAmountInRoute {
                    pool_id: 3,
                    token_out_denom: DENOM_UION.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 2,
                    token_out_denom: DENOM_UOSMO.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: DENOM_UATOM.to_string(),
                }
            ]
        );

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UATOM.to_string()).unwrap(),
            vec![
                SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: DENOM_UOSMO.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 2,
                    token_out_denom: DENOM_UION.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 3,
                    token_out_denom: DENOM_USDC.to_string(),
                },
            ]
        );
    }
}
