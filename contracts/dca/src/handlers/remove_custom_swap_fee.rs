use crate::helpers::validation::assert_sender_is_admin;
use crate::state::config::get_custom_fee;
use crate::{error::ContractError, state::config::remove_custom_fee};
use cosmwasm_std::DepsMut;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{MessageInfo, Response};

pub fn remove_custom_swap_fee_handler(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    let fee = get_custom_fee(deps.storage, denom.clone())?;

    if fee.is_none() {
        return Err(ContractError::CustomError {
            val: format!("Custom fee for {} does not exist", denom),
        });
    }

    remove_custom_fee(deps.storage, denom.clone());

    Ok(Response::new()
        .add_attribute("remove_custom_swap_fee", "true")
        .add_attribute("denom", denom))
}

#[cfg(test)]
mod remove_custom_swap_fee_tests {
    use super::*;
    use crate::{
        handlers::{
            create_custom_swap_fee::create_custom_swap_fee_handler,
            get_custom_swap_fees::get_custom_swap_fees_handler,
        },
        state::pairs::save_pair,
        tests::{
            helpers::instantiate_contract,
            mocks::{ADMIN, DENOM_STAKE},
        },
        types::pair::Pair,
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Decimal,
    };

    #[test]
    fn without_custom_fee_fails() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let denom = DENOM_STAKE.to_string();

        let err = remove_custom_swap_fee_handler(deps.as_mut(), info, denom).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: Custom fee for stake does not exist"
        );
    }

    #[test]
    fn with_custom_fee_succeeds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let denom = DENOM_STAKE.to_string();

        save_pair(
            deps.as_mut().storage,
            &Pair {
                base_denom: denom.to_string(),
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

        remove_custom_swap_fee_handler(deps.as_mut(), info, denom).unwrap();

        let custom_fees = get_custom_swap_fees_handler(deps.as_ref()).unwrap();

        assert_eq!(custom_fees.len(), 0);
    }
}
