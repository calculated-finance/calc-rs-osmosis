use crate::{msg::PairsResponse, state::pairs::get_pairs};
use cosmwasm_std::{Deps, StdResult};

pub fn get_pairs_handler(deps: Deps) -> StdResult<PairsResponse> {
    Ok(PairsResponse {
        pairs: get_pairs(deps.storage),
    })
}

#[cfg(test)]
mod get_pairs_tests {
    use crate::{
        contract::query,
        handlers::create_pair::create_pair_handler,
        msg::{PairsResponse, QueryMsg},
        tests::{
            helpers::instantiate_contract,
            mocks::{calc_mock_dependencies, ADMIN},
        },
        types::pair::Pair,
    };
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
    };

    #[test]
    fn get_all_pairs_with_one_whitelisted_pair_should_succeed() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let pair = Pair::default();

        create_pair_handler(
            deps.as_mut(),
            info.clone(),
            pair.base_denom.clone(),
            pair.quote_denom.clone(),
            pair.route.clone(),
        )
        .unwrap();

        let binary = query(deps.as_ref(), env, QueryMsg::GetPairs {}).unwrap();
        let response = from_binary::<PairsResponse>(&binary).unwrap();

        assert_eq!(response.pairs.len(), 1);
        assert_eq!(response.pairs[0], pair);
    }

    #[test]
    fn get_all_pairs_with_no_whitelisted_pairs_should_succeed() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info);

        let binary = query(deps.as_ref(), env, QueryMsg::GetPairs {}).unwrap();
        let response = from_binary::<PairsResponse>(&binary).unwrap();

        assert_eq!(response.pairs.len(), 0);
    }
}
