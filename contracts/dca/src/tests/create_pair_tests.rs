use crate::{
    contract::execute,
    handlers::get_pairs::get_pairs,
    msg::ExecuteMsg,
    state::pairs::PAIRS,
    tests::{
        helpers::instantiate_contract,
        instantiate_tests::VALID_ADDRESS_ONE,
        mocks::{ADMIN, DENOM_STAKE, DENOM_UOSMO},
    },
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
        address: Addr::unchecked("pair"),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
        route: vec![0],
    };

    execute(deps.as_mut(), env, info, create_pair_execute_message).unwrap();

    let pair = &get_pairs(deps.as_ref()).unwrap().pairs[0];

    assert_eq!(pair.base_denom, "base");
    assert_eq!(pair.quote_denom, "quote");
}

#[test]
fn create_pair_that_already_exists_should_update_it() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let address = Addr::unchecked("pair");

    let original_message = ExecuteMsg::CreatePair {
        address: address.clone(),
        base_denom: DENOM_UOSMO.to_string(),
        quote_denom: DENOM_STAKE.to_string(),
        route: vec![0],
    };

    let message = ExecuteMsg::CreatePair {
        address: address.clone(),
        base_denom: DENOM_UOSMO.to_string(),
        quote_denom: DENOM_STAKE.to_string(),
        route: vec![1],
    };

    execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        original_message.clone(),
    )
    .unwrap();

    execute(deps.as_mut(), env.clone(), info.clone(), original_message).unwrap();

    let original_pair = PAIRS.load(deps.as_ref().storage, address.clone()).unwrap();

    execute(deps.as_mut(), env, info, message).unwrap();

    let pair = PAIRS.load(deps.as_ref().storage, address).unwrap();

    assert_eq!(original_pair.route, vec![0]);
    assert_eq!(pair.route, vec![1]);
}

#[test]
fn create_pair_with_unauthorised_sender_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let info_with_unauthorised_sender = mock_info(VALID_ADDRESS_ONE, &vec![]);

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: Addr::unchecked("pair"),
        base_denom: String::from("base"),
        quote_denom: String::from("quote"),
        route: vec![0],
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
