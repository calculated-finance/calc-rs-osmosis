use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};

use crate::{contract::instantiate, msg::InstantiateMsg, state::get_config};

pub const USER: &str = "user";

#[test]
fn saves_config() {
    let mut deps = mock_dependencies();
    let mock_env = mock_env();
    let info = mock_info(USER, &[]);

    let router = Addr::unchecked("router");
    let swapper = Addr::unchecked("swapper");
    let base_denom = String::from("ukuji");

    let instantiate_msg = InstantiateMsg {
        router: router.clone(),
        swapper: swapper.clone(),
        base_denom: base_denom.clone(),
    };

    instantiate(deps.as_mut(), mock_env.clone(), info, instantiate_msg).unwrap();

    let config = get_config(deps.as_ref().storage).unwrap();

    assert_eq!(config.router, router);
    assert_eq!(config.swapper, swapper);
    assert_eq!(config.base_denom, base_denom);
}
