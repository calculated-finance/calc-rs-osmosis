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
    let pool = get_pool(querier, pool_id)?;

    if pool
        .pool_assets
        .iter()
        .all(|asset| asset.token.clone().unwrap().denom != token_in_denom)
    {
        return Err(StdError::generic_err(format!(
            "denom {} not found in pool id {}",
            token_in_denom, pool_id
        )));
    }

    let token_out_denom = pool
        .pool_assets
        .iter()
        .find(|asset| asset.token.clone().unwrap().denom != token_in_denom)
        .map(|asset| asset.token.clone().unwrap().denom)
        .ok_or_else(|| StdError::generic_err("no token out denom found"));

    token_out_denom
}

pub fn get_pool(querier: &QuerierWrapper, pool_id: u64) -> Result<Pool, StdError> {
    GammQuerier::new(querier)
        .pool(pool_id)?
        .pool
        .unwrap_or_else(|| panic!("pool id {}", pool_id))
        .try_into()
        .map_err(|e: DecodeError| StdError::ParseErr {
            target_type: Pool::TYPE_URL.to_string(),
            msg: e.to_string(),
        })
}

pub fn calculate_route(
    querier: &QuerierWrapper,
    pair: &Pair,
    mut swap_denom: String,
) -> StdResult<Vec<SwapAmountInRoute>> {
    let pair_denoms = pair.denoms();

    if !pair_denoms.contains(&swap_denom) {
        return Err(StdError::generic_err(format!(
            "swap denom {} not in pair denoms {:?}",
            swap_denom, pair_denoms
        )));
    }

    let pool_ids = match pair.position_type(swap_denom.clone()) {
        PositionType::Enter => pair.route.clone(),
        PositionType::Exit => pair.route.clone().into_iter().rev().collect(),
    };

    let mut route: Vec<SwapAmountInRoute> = vec![];

    for pool_id in pool_ids.into_iter() {
        let target_denom = get_token_out_denom(querier, swap_denom.clone(), pool_id)?;

        route.push(SwapAmountInRoute {
            pool_id,
            token_out_denom: target_denom.clone(),
        });

        swap_denom = target_denom;
    }

    if !pair_denoms.contains(&route.last().unwrap().token_out_denom) {
        return Err(StdError::generic_err(format!(
            "last token out denom {} not in pair denoms {:?}",
            route.last().unwrap().token_out_denom,
            pair_denoms
        )));
    }

    Ok(route)
}

#[cfg(test)]
mod get_token_out_denom_tests {
    use super::get_token_out_denom;
    use crate::{
        tests::mocks::{calc_mock_dependencies, DENOM_UATOM, DENOM_UOSMO},
        types::pair::Pair,
    };

    #[test]
    fn fails_when_swap_denom_not_in_pair_denoms() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![0],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UOSMO.to_string(),
            ..Pair::default()
        };

        let swap_denom = "not_in_pair".to_string();

        let err = get_token_out_denom(&deps.as_ref().querier, swap_denom.clone(), pair.route[0])
            .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Generic error: denom {} not found in pool id {}",
                swap_denom, pair.route[0]
            )
        );
    }
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
    fn fails_when_swap_denom_not_in_pair_denoms() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![0],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UOSMO.to_string(),
            ..Pair::default()
        };

        let swap_denom = "not_in_pair".to_string();

        let err = calculate_route(&deps.as_ref().querier, &pair, swap_denom.clone()).unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Generic error: swap denom {} not in pair denoms {:?}",
                swap_denom,
                pair.denoms()
            )
        );
    }

    #[test]
    fn fails_when_initial_pool_does_not_contain_swap_denom() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![2],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UOSMO.to_string(),
            ..Pair::default()
        };

        let err = calculate_route(
            &deps.as_ref().querier,
            &pair.clone(),
            pair.quote_denom.clone(),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Generic error: denom {} not found in pool id {}",
                pair.quote_denom, pair.route[0]
            )
        );
    }

    #[test]
    fn fails_when_intermediary_pool_does_not_contain_target_denom() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![0, 2],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UOSMO.to_string(),
            ..Pair::default()
        };

        let err = calculate_route(
            &deps.as_ref().querier,
            &pair.clone(),
            pair.quote_denom.clone(),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Generic error: denom {} not found in pool id {}",
                pair.base_denom, pair.route[1]
            )
        );
    }

    #[test]
    fn fails_when_final_pool_does_not_contain_target_denom() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![0, 1],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UOSMO.to_string(),
            ..Pair::default()
        };

        let err = calculate_route(
            &deps.as_ref().querier,
            &pair.clone(),
            pair.quote_denom.clone(),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!(
                "Generic error: last token out denom uion not in pair denoms {:?}",
                pair.denoms()
            )
        );
    }

    #[test]
    fn calculates_1_pool_route() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![0],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UOSMO.to_string(),
            ..Pair::default()
        };

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UATOM.to_string()).unwrap(),
            vec![SwapAmountInRoute {
                pool_id: 0,
                token_out_denom: DENOM_UOSMO.to_string(),
            }]
        );

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UOSMO.to_string()).unwrap(),
            vec![SwapAmountInRoute {
                pool_id: 0,
                token_out_denom: DENOM_UATOM.to_string(),
            }]
        );
    }

    #[test]
    fn calculates_2_pool_route() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![0, 1],
            quote_denom: DENOM_UATOM.to_string(),
            base_denom: DENOM_UION.to_string(),
            ..Pair::default()
        };

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UATOM.to_string()).unwrap(),
            vec![
                SwapAmountInRoute {
                    pool_id: 0,
                    token_out_denom: DENOM_UOSMO.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: DENOM_UION.to_string(),
                }
            ]
        );

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UION.to_string()).unwrap(),
            vec![
                SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: DENOM_UOSMO.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 0,
                    token_out_denom: DENOM_UATOM.to_string(),
                }
            ]
        );
    }

    #[test]
    fn calculates_3_pool_route() {
        let deps = calc_mock_dependencies();

        let pair = Pair {
            route: vec![2, 1, 0],
            quote_denom: DENOM_USDC.to_string(),
            base_denom: DENOM_UATOM.to_string(),
            ..Pair::default()
        };

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_USDC.to_string()).unwrap(),
            vec![
                SwapAmountInRoute {
                    pool_id: 2,
                    token_out_denom: DENOM_UION.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: DENOM_UOSMO.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 0,
                    token_out_denom: DENOM_UATOM.to_string(),
                }
            ]
        );

        assert_eq!(
            calculate_route(&deps.as_ref().querier, &pair, DENOM_UATOM.to_string()).unwrap(),
            vec![
                SwapAmountInRoute {
                    pool_id: 0,
                    token_out_denom: DENOM_UOSMO.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 1,
                    token_out_denom: DENOM_UION.to_string(),
                },
                SwapAmountInRoute {
                    pool_id: 2,
                    token_out_denom: DENOM_USDC.to_string(),
                },
            ]
        );
    }
}
