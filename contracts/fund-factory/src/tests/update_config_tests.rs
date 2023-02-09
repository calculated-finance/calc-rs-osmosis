use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};

use crate::{
    contract::{execute, instantiate, query},
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
};

pub const ADMIN: &str = "admin";

#[test]
fn with_valid_admin_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    let instantiate_msg = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        router_code_id: 0,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

    let updated_admin: &str = "updated_admin";

    let update_config_msg = ExecuteMsg::UpdateConfig {
        admin: Some(Addr::unchecked("updated_admin")),
        router_code_id: None,
    };

    execute(deps.as_mut(), env.clone(), info, update_config_msg).unwrap();

    let get_update_config_msg = QueryMsg::GetConfig {};

    let binary = query(deps.as_ref(), env, get_update_config_msg).unwrap();

    let config_response: ConfigResponse = from_binary(&binary).unwrap();

    assert_eq!(config_response.config.admin, Addr::unchecked(updated_admin));
}

#[test]
fn with_invalid_admin_address_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    let instantiate_msg = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        router_code_id: 0,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

    let updated_admin: &str = "";

    let update_config_msg = ExecuteMsg::UpdateConfig {
        admin: Some(Addr::unchecked(updated_admin)),
        router_code_id: None,
    };

    let update_config_res = execute(deps.as_mut(), env.clone(), info, update_config_msg);

    assert!(update_config_res.is_err())
}

#[test]
fn with_valid_code_id_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    let instantiate_msg = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        router_code_id: 0,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

    let updated_code_id: u64 = 1;

    let update_config_msg = ExecuteMsg::UpdateConfig {
        admin: None,
        router_code_id: Some(updated_code_id),
    };

    execute(deps.as_mut(), env.clone(), info, update_config_msg).unwrap();

    let get_update_config_msg = QueryMsg::GetConfig {};

    let binary = query(deps.as_ref(), env, get_update_config_msg).unwrap();

    let config_response: ConfigResponse = from_binary(&binary).unwrap();

    assert_eq!(config_response.config.router_code_id, updated_code_id);
}

#[test]
fn with_no_admin_permissions_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    let instantiate_msg = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        router_code_id: 0,
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

    let unauthorised_admin: &str = "unauthorised_admin";

    let unauthorised_info = mock_info(unauthorised_admin, &vec![]);

    let update_config_msg = ExecuteMsg::UpdateConfig {
        admin: Some(Addr::unchecked(unauthorised_admin)),
        router_code_id: None,
    };

    let update_config_res = execute(
        deps.as_mut(),
        env.clone(),
        unauthorised_info,
        update_config_msg,
    )
    .unwrap_err();

    assert_eq!(update_config_res.to_string(), "Unauthorized");
}
