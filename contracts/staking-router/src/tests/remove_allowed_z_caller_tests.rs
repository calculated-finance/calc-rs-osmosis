use cosmwasm_std::Addr;
use cw_multi_test::Executor;

use crate::msg::{ExecuteMsg, QueryMsg};

use super::mocks::{MockApp, ADMIN};

#[test]
fn with_admin_should_succeed() {
    let existing_z_caller = Addr::unchecked("allowedzcaller".to_string());

    let mut mock = MockApp::new();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.staking_router_contract_address.clone(),
            &ExecuteMsg::RemoveAllowedZCaller {
                allowed_z_caller: existing_z_caller,
            },
            &vec![],
        )
        .unwrap();

    let allowed_z_callers_response: Vec<Addr> = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.staking_router_contract_address,
            &QueryMsg::GetAllowedZCallers {},
        )
        .unwrap();

    assert_eq!(allowed_z_callers_response.len(), 0)
}

#[test]
fn with_admin_and_non_existant_z_caller_should_succeed() {
    let existing_z_caller = Addr::unchecked("doesntexist".to_string());

    let mut mock = MockApp::new();

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.staking_router_contract_address.clone(),
            &ExecuteMsg::RemoveAllowedZCaller {
                allowed_z_caller: existing_z_caller,
            },
            &vec![],
        )
        .unwrap();

    let allowed_z_callers_response: Vec<Addr> = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.staking_router_contract_address,
            &QueryMsg::GetAllowedZCallers {},
        )
        .unwrap();

    assert_eq!(allowed_z_callers_response.len(), 1)
}

#[test]
fn without_admin_should_succeed() {
    let existing_z_caller = Addr::unchecked("allowedzcaller".to_string());

    let mut mock = MockApp::new();

    let res = mock
        .app
        .execute_contract(
            Addr::unchecked("notadmin"),
            mock.staking_router_contract_address.clone(),
            &ExecuteMsg::RemoveAllowedZCaller {
                allowed_z_caller: existing_z_caller,
            },
            &vec![],
        )
        .unwrap_err();

    assert_eq!(res.root_cause().to_string(), "Unauthorized")
}
