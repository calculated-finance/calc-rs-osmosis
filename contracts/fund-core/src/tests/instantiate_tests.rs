use crate::{contract::instantiate, msg::InstantiateMsg, state::get_config};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};

use crate::{
    tests::helpers::{BASE_DENOM, ROUTER_ADDRESS, SWAPPER_ADDRESS},
};

#[test]
fn saves_config() {
    let mut deps = mock_dependencies();
    let mock_env = mock_env();
    let info = mock_info("factory", &[]);

    let router = Addr::unchecked(ROUTER_ADDRESS);
    let swapper = Addr::unchecked(SWAPPER_ADDRESS);
    let base_denom = BASE_DENOM.to_string();

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
