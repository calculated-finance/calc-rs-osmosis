use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};

use crate::contract::instantiate;
use crate::msg::InstantiateMsg;

pub const ADMIN: &str = "admin";

#[test]
fn with_valid_admin_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    let msg = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        router_code_id: 0,
    };

    let res = instantiate(deps.as_mut(), env, info, msg);

    assert!(res.is_ok())
}

#[test]
fn with_invalid_admin_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    let msg = InstantiateMsg {
        admin: Addr::unchecked(""),
        router_code_id: 0,
    };

    let res = instantiate(deps.as_mut(), env, info, msg);

    assert!(res.is_err())
}

#[test]
fn with_valid_code_id_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    let msg = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        router_code_id: 1,
    };

    let res = instantiate(deps.as_mut(), env, info, msg);

    assert!(res.is_ok())
}
