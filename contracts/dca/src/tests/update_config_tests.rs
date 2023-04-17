use super::{helpers::instantiate_contract, mocks::ADMIN};
use crate::{
    handlers::update_config::update_config_handler,
    state::config::{get_config, FeeCollector},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Decimal,
};
use std::str::FromStr;

#[test]
fn update_swap_fee_percent_with_valid_value_should_succeed() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    update_config_handler(
        deps.as_mut(),
        info,
        None,
        Some(Decimal::from_str("0.1").unwrap()),
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let config = get_config(deps.as_ref().storage).unwrap();

    assert_eq!(config.swap_fee_percent, Decimal::percent(10));
}

#[test]
fn update_swap_fee_percent_more_than_100_percent_should_fail() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    let err = update_config_handler(
        deps.as_mut(),
        info,
        None,
        Some(Decimal::percent(150)),
        None,
        None,
        None,
        None,
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Generic error: swap_fee_percent must be less than 100%, and expressed as a ratio out of 1 (i.e. use 0.015 to represent a fee of 1.5%)"
    )
}

#[test]
fn update_delegation_fee_percent_with_valid_value_should_succeed() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    update_config_handler(
        deps.as_mut(),
        info,
        None,
        None,
        Some(Decimal::from_str("0.1").unwrap()),
        None,
        None,
        None,
    )
    .unwrap();

    let config = get_config(deps.as_ref().storage).unwrap();

    assert_eq!(config.delegation_fee_percent, Decimal::percent(10));
}

#[test]
fn update_delegation_fee_percent_more_than_100_percent_should_fail() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    let err = update_config_handler(
        deps.as_mut(),
        info,
        None,
        None,
        Some(Decimal::percent(150)),
        None,
        None,
        None,
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Generic error: delegation_fee_percent must be less than 100%, and expressed as a ratio out of 1 (i.e. use 0.015 to represent a fee of 1.5%)"
    )
}

#[test]
fn update_fee_collectors_with_no_value_should_not_change_value() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    let config_before_update = get_config(deps.as_ref().storage).unwrap();

    update_config_handler(deps.as_mut(), info, None, None, None, None, None, None).unwrap();

    let config_after_update = get_config(deps.as_ref().storage).unwrap();

    assert_eq!(
        config_after_update.fee_collectors,
        config_before_update.fee_collectors
    );
}

#[test]
fn update_fee_collectors_with_valid_value_should_succeed() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    let fee_collectors = Some(vec![
        FeeCollector {
            address: ADMIN.to_string(),
            allocation: Decimal::from_str("0.9").unwrap(),
        },
        FeeCollector {
            address: ADMIN.to_string(),
            allocation: Decimal::from_str("0.1").unwrap(),
        },
    ]);

    update_config_handler(
        deps.as_mut(),
        info,
        fee_collectors.clone(),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let config = get_config(deps.as_ref().storage).unwrap();

    assert_eq!(config.fee_collectors, fee_collectors.unwrap());
}

#[test]
fn update_fee_collectors_with_total_allocations_more_than_100_percent_should_fail() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    let err = update_config_handler(
        deps.as_mut(),
        info,
        Some(vec![
            FeeCollector {
                address: ADMIN.to_string(),
                allocation: Decimal::from_str("1").unwrap(),
            },
            FeeCollector {
                address: ADMIN.to_string(),
                allocation: Decimal::from_str("1").unwrap(),
            },
        ]),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: fee collector allocations must add up to 1"
    )
}

#[test]
fn update_dca_plus_escrow_level_with_valid_value_should_succeed() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    update_config_handler(
        deps.as_mut(),
        info,
        None,
        None,
        None,
        None,
        None,
        Some(Decimal::percent(19)),
    )
    .unwrap();

    let config = get_config(deps.as_ref().storage).unwrap();

    assert_eq!(config.dca_plus_escrow_level, Decimal::percent(19));
}

#[test]
fn update_dca_plus_escrow_level_more_than_100_percent_should_fail() {
    let mut deps = mock_dependencies();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), mock_env(), info.clone());

    let err = update_config_handler(
        deps.as_mut(),
        info,
        None,
        None,
        None,
        None,
        None,
        Some(Decimal::percent(150)),
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: dca_plus_escrow_level cannot be greater than 100%"
    )
}
