use crate::{
    contract::query,
    handlers::create_pair::create_pair,
    msg::{PairsResponse, QueryMsg},
    tests::{helpers::instantiate_contract, mocks::ADMIN},
    types::pair::Pair,
};
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};

#[test]
fn get_all_pairs_with_one_whitelisted_pair_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pair(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        Addr::unchecked("pair"),
        "base".to_string(),
        "quote".to_string(),
    )
    .unwrap();

    let get_all_pairs_query_message = QueryMsg::GetPairs {};

    let binary = query(deps.as_ref(), env, get_all_pairs_query_message).unwrap();
    let response = from_binary::<PairsResponse>(&binary).unwrap();

    assert_eq!(response.pairs.len(), 1);
    assert_eq!(
        response.pairs[0],
        Pair {
            pool_id: 0,
            address: Addr::unchecked("pair"),
            base_denom: "base".to_string(),
            quote_denom: "quote".to_string(),
        }
    );
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
