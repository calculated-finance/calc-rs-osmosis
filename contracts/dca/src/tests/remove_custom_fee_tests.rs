use super::mocks::ADMIN;
use crate::{
    handlers::{
        create_custom_swap_fee::create_custom_swap_fee, get_custom_swap_fees::get_custom_swap_fees,
        remove_custom_swap_fee::remove_custom_swap_fee,
    },
    tests::{helpers::instantiate_contract, mocks::DENOM_STAKE},
};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Decimal,
};

#[test]
fn remove_custom_fee_should_succeed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let denom = DENOM_STAKE.to_string();

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

    remove_custom_swap_fee(deps.as_mut(), info, denom).unwrap();

    let custom_fees = get_custom_swap_fees(deps.as_ref()).unwrap();

    assert_eq!(custom_fees.len(), 0);
}
