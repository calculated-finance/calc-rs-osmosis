use std::str::FromStr;

use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Decimal,
};

use crate::{
    contract::{instantiate, query},
    msg::{ConfigResponse, InstantiateMsg, QueryMsg},
    state::config::FeeCollector,
    tests::mocks::ADMIN,
};

#[test]
fn get_config_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        fee_collectors: vec![FeeCollector {
            address: Addr::unchecked("fee-collector"),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked("staking-router"),
        page_limit: 1000,
        paused: false,
    };

    let _instantiate_result = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        instantiate_message,
    )
    .unwrap();

    let get_config_query_msg = QueryMsg::GetConfig {};
    let binary = query(deps.as_ref(), env, get_config_query_msg).unwrap();
    let response: ConfigResponse = from_binary(&binary).unwrap();
    assert_eq!(response.config.admin, Addr::unchecked(ADMIN));
    assert_eq!(
        response.config.fee_collectors[0].address,
        Addr::unchecked("fee-collector")
    );
    assert_eq!(
        response.config.swap_fee_percent,
        Decimal::from_str("0.015").unwrap()
    );
    assert_eq!(
        response.config.delegation_fee_percent,
        Decimal::from_str("0.0075").unwrap()
    );
    assert_eq!(
        response.config.staking_router_address,
        Addr::unchecked("staking-router")
    );
    assert_eq!(response.config.page_limit, 1000);
    assert_eq!(response.config.paused, false);
}
