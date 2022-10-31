use std::str::FromStr;

use cosmwasm_std::{Addr, Decimal};
use cw_multi_test::Executor;

use crate::msg::ExecuteMsg;

use super::mocks::{fin_contract_unfilled_limit_order, MockApp, ADMIN};

#[test]
fn update_fee_percent_with_valid_value_should_succeed() {
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order());

    mock.app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::UpdateConfig {
                fee_collector: Some(Addr::unchecked(ADMIN)),
                fee_percent: Some(Decimal::from_str("0.015").unwrap()),
                staking_router_address: None,
                page_limit: None,
            },
            &[],
        )
        .unwrap();
}

#[test]
fn update_fee_percent_more_than_100_percent_should_fail() {
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order());

    let error = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::UpdateConfig {
                fee_collector: Some(Addr::unchecked(ADMIN)),
                fee_percent: Some(Decimal::from_str("1.5").unwrap()),
                staking_router_address: None,
                page_limit: None,
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        error.root_cause().to_string(),
        "Generic error: fee_percent must be less than 100% (i.e. 0.015)"
    )
}
