use crate::{
    contract::execute,
    handlers::get_pools::get_pools,
    msg::ExecuteMsg,
    tests::{helpers::instantiate_contract, instantiate_tests::VALID_ADDRESS_ONE, mocks::ADMIN},
};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

#[test]
fn create_pool_with_valid_id_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let create_pool_execute_message = ExecuteMsg::CreatePool {
        pool_id: 0,
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    execute(deps.as_mut(), env, info, create_pool_execute_message).unwrap();

    let pool = &get_pools(deps.as_ref()).unwrap().pools[0];

    assert_eq!(pool.pool_id, 0);
    assert_eq!(pool.base_denom, "base");
    assert_eq!(pool.quote_denom, "quote");
}

#[test]
fn create_pool_that_already_exists_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let message = ExecuteMsg::CreatePool {
        pool_id: 0,
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), message.clone()).unwrap();

    let result = execute(deps.as_mut(), env, info, message).unwrap_err();

    assert_eq!(result.to_string(), "Error: pool already exists for id 0")
}

#[test]
fn create_pool_with_unauthorised_sender_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let info_with_unauthorised_sender = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let create_pool_execute_message = ExecuteMsg::CreatePool {
        pool_id: 0,
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let result = execute(
        deps.as_mut(),
        env,
        info_with_unauthorised_sender,
        create_pool_execute_message,
    )
    .unwrap_err();

    assert_eq!(result.to_string(), "Unauthorized")
}
