use crate::contract::instantiate;
use crate::msg::InstantiateMsg;
use crate::state::config::FeeCollector;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, Addr, Decimal};
use std::str::FromStr;

pub const INVALID_ADDRESS: &str = "";
pub const VALID_ADDRESS_ONE: &str = "osmo16q6jpx7ns0ugwghqay73uxd5aq30du3uqgxf0d";
pub const VALID_ADDRESS_TWO: &str = "osmo1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lfv";
pub const VALID_ADDRESS_THREE: &str = "osmo1cvlzqz80rp70xtmux9x69j4sr0rndh3yws2lf1";

#[test]
fn instantiate_with_valid_admin_address_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("method", "instantiate"),
            attr("admin", "osmo16q6jpx7ns0ugwghqay73uxd5aq30du3uqgxf0d")
        ]
    )
}

#[test]
fn instantiate_with_invalid_admin_address_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(INVALID_ADDRESS),
        fee_collectors: vec![FeeCollector {
            address: VALID_ADDRESS_ONE.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Generic error: Invalid input: human address too short for this mock implementation (must be >= 3)."
    )
}

#[test]
fn instantiate_with_invalid_fee_collector_address_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![FeeCollector {
            address: INVALID_ADDRESS.to_string(),
            allocation: Decimal::from_str("1").unwrap(),
        }],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: fee collector address  is invalid"
    )
}

#[test]
fn instantiate_with_fee_collector_amounts_not_equal_to_100_percent_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("", &vec![]);

    let instantiate_message = InstantiateMsg {
        admin: Addr::unchecked(VALID_ADDRESS_ONE),
        fee_collectors: vec![],
        swap_fee_percent: Decimal::from_str("0.015").unwrap(),
        delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
        staking_router_address: Addr::unchecked(VALID_ADDRESS_ONE),
        page_limit: 1000,
        paused: false,
        dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
    };

    let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

    assert_eq!(
        result.to_string(),
        "Error: fee collector allocations must add up to 1"
    )
}
