use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, BankMsg, Coin, SubMsg, Uint128,
};

use super::helpers::{instantiate_contract, USER};
use crate::{contract::execute, msg::ExecuteMsg, tests::helpers::ROUTER_ADDRESS};

#[test]
fn should_transfer_funds_to_new_contract_address() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ROUTER_ADDRESS, &vec![]);
    let new_fund_address = Addr::unchecked("new_contract_address".to_string());

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(Uint128::new(100).into(), "usdc".to_string())],
    );

    let migrate_msg = ExecuteMsg::Migrate {
        new_fund_address: new_fund_address.clone(),
    };

    let response = execute(deps.as_mut(), env, info, migrate_msg).unwrap();

    assert!(response.messages.contains(&SubMsg::new(BankMsg::Send {
        to_address: new_fund_address.to_string(),
        amount: vec![Coin::new(Uint128::new(100).into(), "usdc".to_string(),)]
    })));
}

#[test]
fn with_no_funds_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ROUTER_ADDRESS, &vec![]);
    let new_fund_address = Addr::unchecked("new_contract_address".to_string());

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let migrate_msg = ExecuteMsg::Migrate {
        new_fund_address: new_fund_address.clone(),
    };

    let response = execute(deps.as_mut(), env, info, migrate_msg).unwrap();

    assert!(response.messages.is_empty());
}

#[test]
fn with_unauthorised_sender_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ROUTER_ADDRESS, &vec![]);
    let new_fund_address = Addr::unchecked("new_contract_address".to_string());

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    deps.querier.update_balance(
        "cosmos2contract",
        vec![Coin::new(Uint128::new(100).into(), "usdc".to_string())],
    );

    let unauthorised_info = mock_info(USER, &vec![]);

    let migrate_msg = ExecuteMsg::Migrate {
        new_fund_address: new_fund_address.clone(),
    };

    let response = execute(deps.as_mut(), env, unauthorised_info, migrate_msg);

    assert_eq!(response.unwrap_err().to_string(), "Unauthorized")
}
