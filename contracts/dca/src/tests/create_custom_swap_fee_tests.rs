use super::mocks::ADMIN;
use crate::{
    handlers::{
        create_custom_swap_fee::create_custom_swap_fee, get_custom_swap_fees::get_custom_swap_fees,
    },
    tests::helpers::instantiate_contract,
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Decimal,
};

#[test]
fn create_custom_swap_fee_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let denom = "ukuji".to_string();

    create_custom_swap_fee(deps.as_mut(), info, denom.clone(), Decimal::percent(1)).unwrap();

    let custom_fees = get_custom_swap_fees(deps.as_ref()).unwrap();

    assert_eq!(custom_fees.len(), 1);
    assert_eq!(custom_fees[0], (denom.clone(), Decimal::percent(1)));
}

#[test]
fn create_custom_swap_fee_should_overwrite_existing_fee() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let denom = "ukuji".to_string();

    create_custom_swap_fee(
        deps.as_mut(),
        info.clone(),
        denom.clone(),
        Decimal::percent(1),
    )
    .unwrap();

    let custom_fees = get_custom_swap_fees(deps.as_ref()).unwrap();

    assert_eq!(custom_fees.len(), 1);
    assert_eq!(custom_fees[0], (denom.clone(), Decimal::percent(1)));

    create_custom_swap_fee(deps.as_mut(), info, denom.clone(), Decimal::percent(3)).unwrap();

    let custom_fees = get_custom_swap_fees(deps.as_ref()).unwrap();

    assert_eq!(custom_fees.len(), 1);
    assert_eq!(custom_fees[0], (denom, Decimal::percent(3)));
}

#[test]
fn create_custom_swap_fee_larger_than_100_percent_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);
    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let response = create_custom_swap_fee(
        deps.as_mut(),
        info,
        "ukuji".to_string(),
        Decimal::percent(101),
    )
    .unwrap_err();

    assert_eq!(response.to_string(), "Generic error: swap_fee_percent must be less than 100%, and expressed as a ratio out of 1 (i.e. use 0.015 to represent a fee of 1.5%)");
}
