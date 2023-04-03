use crate::{
    contract::query,
    handlers::create_pool::create_pool,
    msg::{PoolsResponse, QueryMsg},
    tests::{helpers::instantiate_contract, mocks::ADMIN},
};
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
};

#[test]
fn get_all_pairs_with_one_whitelisted_pair_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    create_pool(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        0,
        "base".to_string(),
        "quote".to_string(),
    )
    .unwrap();

    let get_all_pairs_query_message = QueryMsg::GetPools {};

    let binary = query(deps.as_ref(), env, get_all_pairs_query_message).unwrap();
    let response = from_binary::<PoolsResponse>(&binary).unwrap();

    assert_eq!(response.pools.len(), 1);
    assert_eq!(response.pools[0].pool_id, 0);
    assert_eq!(response.pools[0].base_denom, "base".to_string());
    assert_eq!(response.pools[0].quote_denom, "quote".to_string());
}

#[test]
fn get_all_pairs_with_no_whitelisted_pairs_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info);

    let get_all_pairs_query_message = QueryMsg::GetPools {};
    let binary = query(deps.as_ref(), env, get_all_pairs_query_message).unwrap();
    let response = from_binary::<PoolsResponse>(&binary).unwrap();

    assert_eq!(response.pools.len(), 0);
}
