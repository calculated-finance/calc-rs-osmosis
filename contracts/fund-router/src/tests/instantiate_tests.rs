use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, SubMsg,
};
use kujira::{denom::Denom, msg::DenomMsg};

use crate::{
    contract::instantiate, msg::InstantiateMsg, state::config::get_config, state::funds::FUNDS,
};

pub const USER: &str = "user";

#[test]
fn creates_new_denom() {
    let mut deps = mock_dependencies();
    let mock_env = mock_env();
    let info = mock_info(USER, &[]);

    let instantiate_msg = InstantiateMsg {
        token_name: "test_token".to_string(),
        owner: Addr::unchecked(USER),
    };

    let response = instantiate(deps.as_mut(), mock_env.clone(), info, instantiate_msg).unwrap();

    assert!(response.messages.contains(&SubMsg::new(DenomMsg::Create {
        subdenom: Denom::from("test_token"),
    })));
}

#[test]
fn initialises_funds() {
    let mut deps = mock_dependencies();
    let mock_env = mock_env();
    let info = mock_info(USER, &[]);

    let instantiate_msg = InstantiateMsg {
        token_name: "test_token".to_string(),
        owner: Addr::unchecked(USER),
    };

    instantiate(deps.as_mut(), mock_env.clone(), info, instantiate_msg).unwrap();

    assert!(FUNDS.load(deps.as_mut().storage).is_ok());
}

#[test]
fn with_valid_owner_should_succeed() {
    let mut deps = mock_dependencies();
    let mock_env = mock_env();
    let info = mock_info(USER, &[]);

    let instantiate_msg = InstantiateMsg {
        token_name: "test_token".to_string(),
        owner: Addr::unchecked(USER),
    };

    instantiate(deps.as_mut(), mock_env.clone(), info, instantiate_msg).unwrap();

    let config = get_config(deps.as_ref().storage).unwrap();

    assert_eq!(config.owner, Addr::unchecked(USER));
}

#[test]
fn should_set_sender_as_factory() {
    let mut deps = mock_dependencies();
    let mock_env = mock_env();

    let info = mock_info(USER, &[]);

    let instantiate_msg = InstantiateMsg {
        token_name: "test_token".to_string(),
        owner: Addr::unchecked(USER),
    };

    instantiate(deps.as_mut(), mock_env.clone(), info, instantiate_msg).unwrap();

    let config = get_config(deps.as_ref().storage).unwrap();

    assert_eq!(config.factory, Addr::unchecked(USER));
}
