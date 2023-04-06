use crate::{
    contract::execute,
    handlers::get_pairs::get_pairs,
    msg::ExecuteMsg,
    tests::{helpers::instantiate_contract, instantiate_tests::VALID_ADDRESS_ONE, mocks::ADMIN},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};

#[test]
fn create_pair_with_valid_id_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        pool_id: 0,
        address: Addr::unchecked("pair"),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    execute(deps.as_mut(), env, info, create_pair_execute_message).unwrap();

    let pair = &get_pairs(deps.as_ref()).unwrap().pairs[0];

    assert_eq!(pair.pool_id, 0);
    assert_eq!(pair.base_denom, "base");
    assert_eq!(pair.quote_denom, "quote");
}

#[test]
fn create_pair_that_already_exists_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let message = ExecuteMsg::CreatePair {
        pool_id: 0,
        address: Addr::unchecked("pair"),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), message.clone()).unwrap();

    let result = execute(deps.as_mut(), env, info, message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: pair already exists for address pair"
    )
}

#[test]
fn create_pair_with_unauthorised_sender_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let info_with_unauthorised_sender = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        pool_id: 0,
        address: Addr::unchecked("pair"),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
    };

    let result = execute(
        deps.as_mut(),
        env,
        info_with_unauthorised_sender,
        create_pair_execute_message,
    )
    .unwrap_err();

    assert_eq!(result.to_string(), "Unauthorized")
}
