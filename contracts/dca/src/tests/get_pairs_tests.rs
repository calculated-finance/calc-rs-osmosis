use crate::{
    contract::query,
    handlers::create_pair::create_pair,
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

    create_pair(
        deps.as_mut(),
        info.clone(),
        pair.address.clone(),
        pair.base_denom.clone(),
        pair.quote_denom.clone(),
        pair.route.clone(),
    )
    .unwrap();

    let get_all_pairs_query_message = QueryMsg::GetPairs {};

    let binary = query(deps.as_ref(), env, get_all_pairs_query_message).unwrap();
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

    let get_all_pairs_query_message = QueryMsg::GetPairs {};
    let binary = query(deps.as_ref(), env, get_all_pairs_query_message).unwrap();
    let response = from_binary::<PairsResponse>(&binary).unwrap();

    assert_eq!(response.pairs.len(), 0);
}
