use crate::helpers::validation::assert_sender_is_admin;
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

    remove_custom_fee(deps.storage, denom.clone());

    Ok(Response::new()
        .add_attribute("method", "remove_custom_swap_fee")
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
        tests::{
            helpers::instantiate_contract,
            mocks::{ADMIN, DENOM_STAKE},
        },
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
