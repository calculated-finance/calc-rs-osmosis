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
                swap_fee_percent: Some(Decimal::from_str("0.015").unwrap()),
                delegation_fee_percent: Some(Decimal::from_str("0.0075").unwrap()),
                staking_router_address: None,
                page_limit: None,
                paused: None,
            },
            &[],
        )
        .unwrap();
}

#[test]
fn update_swap_fee_percent_more_than_100_percent_should_fail() {
    let mut mock = MockApp::new(fin_contract_unfilled_limit_order());

    let error = mock
        .app
        .execute_contract(
            Addr::unchecked(ADMIN),
            mock.dca_contract_address.clone(),
            &ExecuteMsg::UpdateConfig {
                fee_collector: Some(Addr::unchecked(ADMIN)),
                swap_fee_percent: Some(Decimal::from_str("1.5").unwrap()),
                delegation_fee_percent: Some(Decimal::from_str("0.0075").unwrap()),
                staking_router_address: None,
                page_limit: None,
                paused: None,
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        error.root_cause().to_string(),
        "Generic error: swap_fee_percent must be less than 100%, and expressed as a ratio out of 1 (i.e. use 0.015 to represent a fee of 1.5%)"
    )
}
