use crate::{
    contract::query,
    msg::{ConfigResponse, QueryMsg},
    state::config::{Config, FeeCollector},
    tests::{helpers::instantiate_contract, mocks::ADMIN},
};
use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Decimal,
};
use std::str::FromStr;

#[test]
fn get_config_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let get_config_query_msg = QueryMsg::GetConfig {};
    let binary = query(deps.as_ref(), env, get_config_query_msg).unwrap();
    let config = from_binary::<ConfigResponse>(&binary).unwrap().config;

    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN),
            fee_collectors: vec![FeeCollector {
                address: "admin".to_string(),
                allocation: Decimal::percent(100)
            }],
            swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
            delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            staking_router_address: Addr::unchecked("staking-router"),
            page_limit: 1000,
            paused: false,
            dca_plus_escrow_level: Decimal::from_str("0.0075").unwrap(),
        }
    );
}
