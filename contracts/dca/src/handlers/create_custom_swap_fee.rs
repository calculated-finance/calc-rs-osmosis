use crate::{
    error::ContractError,
    helpers::validation::{assert_denom_exists, assert_fee_level_is_valid, assert_sender_is_admin},
    state::config::create_custom_fee,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::Response;
use cosmwasm_std::{Decimal, DepsMut, MessageInfo};

pub fn create_custom_swap_fee_handler(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    swap_fee_percent: Decimal,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;
    assert_denom_exists(deps.as_ref().storage, denom.clone())?;
    assert_fee_level_is_valid(&swap_fee_percent)?;

    create_custom_fee(deps.storage, denom.clone(), swap_fee_percent)?;

    Ok(Response::new()
        .add_attribute("create_custom_swap_fee", "true")
        .add_attribute("denom", denom)
        .add_attribute("swap_fee_percent", swap_fee_percent.to_string()))
}

#[cfg(test)]
mod create_custom_swap_fee_tests {
    use super::*;
    use crate::{
        handlers::get_custom_swap_fees::get_custom_swap_fees_handler,
        state::pairs::save_pair,
        tests::{
            helpers::instantiate_contract,
            mocks::{ADMIN, DENOM_STAKE, DENOM_UATOM, DENOM_UOSMO},
        },
        types::pair::Pair,
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

        let denom = DENOM_UOSMO.to_string();

        save_pair(
            deps.as_mut().storage,
            &Pair {
                base_denom: denom.clone(),
                quote_denom: DENOM_STAKE.to_string(),
                route: vec![1],
            },
        )
        .unwrap();

        create_custom_swap_fee_handler(deps.as_mut(), info, denom.clone(), Decimal::percent(1))
            .unwrap();

        let custom_fees = get_custom_swap_fees_handler(deps.as_ref()).unwrap();

        assert_eq!(custom_fees.len(), 1);
        assert_eq!(custom_fees[0], (denom.clone(), Decimal::percent(1)));
    }

    #[test]
    fn create_custom_swap_fee_should_overwrite_existing_fee() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);
        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let denom = DENOM_UOSMO.to_string();

        save_pair(
            deps.as_mut().storage,
            &Pair {
                base_denom: denom.clone(),
                quote_denom: DENOM_STAKE.to_string(),
                route: vec![1],
            },
        )
        .unwrap();

        create_custom_swap_fee_handler(
            deps.as_mut(),
            info.clone(),
            denom.clone(),
            Decimal::percent(1),
        )
        .unwrap();

        let custom_fees = get_custom_swap_fees_handler(deps.as_ref()).unwrap();

        assert_eq!(custom_fees.len(), 1);
        assert_eq!(custom_fees[0], (denom.clone(), Decimal::percent(1)));

        create_custom_swap_fee_handler(deps.as_mut(), info, denom.clone(), Decimal::percent(3))
            .unwrap();

        let custom_fees = get_custom_swap_fees_handler(deps.as_ref()).unwrap();

        assert_eq!(custom_fees.len(), 1);
        assert_eq!(custom_fees[0], (denom, Decimal::percent(3)));
    }

    #[test]
    fn create_custom_swap_fee_larger_than_5_percent_fails() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);
        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let denom = DENOM_UOSMO.to_string();

        save_pair(
            deps.as_mut().storage,
            &Pair {
                base_denom: denom.to_string(),
                quote_denom: DENOM_STAKE.to_string(),
                route: vec![1],
            },
        )
        .unwrap();

        let response =
            create_custom_swap_fee_handler(deps.as_mut(), info, denom, Decimal::percent(6))
                .unwrap_err();

        assert_eq!(
            response.to_string(),
            "Error: fee level cannot be larger than 5%"
        );
    }

    #[test]
    fn crete_custom_swap_fee_for_unsupported_denom_fails() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);
        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let denom = DENOM_UOSMO.to_string();

        save_pair(
            deps.as_mut().storage,
            &Pair {
                base_denom: DENOM_UATOM.to_string(),
                quote_denom: DENOM_STAKE.to_string(),
                route: vec![1],
            },
        )
        .unwrap();

        let response =
            create_custom_swap_fee_handler(deps.as_mut(), info, denom, Decimal::percent(2))
                .unwrap_err();

        assert_eq!(response.to_string(), "Error: uosmo is not supported");
    }
}
