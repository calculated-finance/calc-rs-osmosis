use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, CosmosMsg, SubMsg,
    WasmMsg::Execute as WasmExecuteMsg,
};

use crate::{
    contract::{execute, query},
    msg::{ExecuteMsg, FundResponse, QueryMsg},
    tests::helpers::ADMIN,
};

use fund_core::msg::ExecuteMsg as FundExecuteMsg;

use super::helpers::{instantiate_contract, FUND_ADDRESS, USER};

#[test]
fn with_valid_address_should_save_fund_address() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let assign_fund_msg = ExecuteMsg::AssignFund {
        fund_address: Addr::unchecked(FUND_ADDRESS),
    };

    execute(deps.as_mut(), env.clone(), info, assign_fund_msg).unwrap();

    let get_fund_query = QueryMsg::GetFund {};

    let binary = query(deps.as_ref(), env, get_fund_query).unwrap();

    let fund_response: FundResponse = from_binary(&binary).unwrap();

    assert_eq!(
        fund_response.address.unwrap(),
        Addr::unchecked(FUND_ADDRESS)
    );
}

#[test]
fn with_invalid_address_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let assign_fund_msg = ExecuteMsg::AssignFund {
        fund_address: Addr::unchecked(""),
    };

    let response = execute(deps.as_mut(), env.clone(), info, assign_fund_msg);

    assert!(response.is_err());
}

#[test]
fn multiple_funds_returns_the_latest_fund() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let assign_fund_msg = ExecuteMsg::AssignFund {
        fund_address: Addr::unchecked(FUND_ADDRESS),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), assign_fund_msg).unwrap();

    let assign_fund_msg = ExecuteMsg::AssignFund {
        fund_address: Addr::unchecked("fund_address_2"),
    };

    execute(deps.as_mut(), env.clone(), info, assign_fund_msg).unwrap();

    let get_fund_query = QueryMsg::GetFund {};

    let binary = query(deps.as_ref(), env, get_fund_query).unwrap();

    let fund_response: FundResponse = from_binary(&binary).unwrap();

    assert_eq!(
        fund_response.address.unwrap(),
        Addr::unchecked("fund_address_2")
    );
}

#[test]
fn without_permission_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let assign_fund_msg = ExecuteMsg::AssignFund {
        fund_address: Addr::unchecked(FUND_ADDRESS),
    };

    let unauthorized_info = mock_info("unauthorized", &vec![]);

    let response = execute(
        deps.as_mut(),
        env.clone(),
        unauthorized_info,
        assign_fund_msg,
    )
    .unwrap_err();

    assert_eq!(response.to_string(), "Unauthorized");
}

#[test]
fn with_existing_fund_should_migrate() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let assign_fund_msg = ExecuteMsg::AssignFund {
        fund_address: Addr::unchecked(FUND_ADDRESS),
    };

    execute(deps.as_mut(), env.clone(), info.clone(), assign_fund_msg).unwrap();

    let assign_fund_msg = ExecuteMsg::AssignFund {
        fund_address: Addr::unchecked("fund_address_2"),
    };

    let res = execute(deps.as_mut(), env.clone(), info, assign_fund_msg).unwrap();

    assert!(res
        .messages
        .contains(&SubMsg::new(CosmosMsg::Wasm(WasmExecuteMsg {
            contract_addr: FUND_ADDRESS.to_string(),
            funds: vec![],
            msg: to_binary(&FundExecuteMsg::Migrate {
                new_fund_address: Addr::unchecked("fund_address_2"),
            })
            .unwrap(),
        }))));
}

#[test]
fn without_existing_fund_should_not_migrate() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(USER, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let assign_fund_msg = ExecuteMsg::AssignFund {
        fund_address: Addr::unchecked(FUND_ADDRESS),
    };

    let res = execute(deps.as_mut(), env.clone(), info, assign_fund_msg).unwrap();

    assert!(res.messages.is_empty());
}
