use crate::helpers::validation::{
    assert_route_matches_denoms, assert_route_not_empty, assert_sender_is_admin,
};
use crate::state::pairs::save_pair;
use crate::{error::ContractError, types::pair::Pair};
use cosmwasm_std::DepsMut;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{MessageInfo, Response};

pub fn create_pair_handler(
    deps: DepsMut,
    info: MessageInfo,
    base_denom: String,
    quote_denom: String,
    route: Vec<u64>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;
    assert_route_not_empty(route.clone())?;

    let pair = Pair {
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        route: route.clone(),
    };

    assert_route_matches_denoms(&deps.querier, &pair)?;

    save_pair(deps.storage, &pair)?;

    Ok(Response::new()
        .add_attribute("method", "create_pair")
        .add_attribute("base_denom", base_denom)
        .add_attribute("quote_denom", quote_denom)
        .add_attribute("route", format!("{:#?}", route)))
}

#[cfg(test)]
mod create_pair_tests {
    use crate::{
        contract::execute,
        handlers::get_pairs::get_pairs_handler,
        msg::ExecuteMsg,
        state::pairs::find_pair,
        tests::{
            helpers::instantiate_contract,
            mocks::{calc_mock_dependencies, ADMIN, DENOM_STAKE, DENOM_UOSMO},
        },
    };
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[test]
    fn create_pair_with_valid_id_should_succeed() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let create_pair_execute_message = ExecuteMsg::CreatePair {
            base_denom: DENOM_UOSMO.to_string(),
            quote_denom: DENOM_STAKE.to_string(),
            route: vec![3],
        };

        execute(deps.as_mut(), env, info, create_pair_execute_message).unwrap();

        let pair = &get_pairs_handler(deps.as_ref()).unwrap().pairs[0];

        assert_eq!(pair.base_denom, DENOM_UOSMO.to_string());
        assert_eq!(pair.quote_denom, DENOM_STAKE.to_string());
        assert_eq!(pair.route, vec![3]);
    }

    #[test]
    fn create_pair_that_already_exists_should_update_it() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let original_message = ExecuteMsg::CreatePair {
            base_denom: DENOM_UOSMO.to_string(),
            quote_denom: DENOM_STAKE.to_string(),
            route: vec![4, 1],
        };

        let message = ExecuteMsg::CreatePair {
            base_denom: DENOM_UOSMO.to_string(),
            quote_denom: DENOM_STAKE.to_string(),
            route: vec![3],
        };

        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            original_message.clone(),
        )
        .unwrap();

        execute(deps.as_mut(), env.clone(), info.clone(), original_message).unwrap();

        let original_pair = find_pair(
            deps.as_ref().storage,
            &[DENOM_UOSMO.to_string(), DENOM_STAKE.to_string()],
        )
        .unwrap();

        execute(deps.as_mut(), env, info, message).unwrap();

        let pair = find_pair(
            deps.as_ref().storage,
            &[DENOM_UOSMO.to_string(), DENOM_STAKE.to_string()],
        )
        .unwrap();

        assert_eq!(original_pair.route, vec![4, 1]);
        assert_eq!(pair.route, vec![3]);
    }

    #[test]
    fn create_pair_with_unauthorised_sender_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let info_with_unauthorised_sender = mock_info("not-admin", &vec![]);

        let create_pair_execute_message = ExecuteMsg::CreatePair {
            base_denom: String::from("base"),
            quote_denom: String::from("quote"),
            route: vec![0],
        };

        let result = execute(
            deps.as_mut(),
            env,
            info_with_unauthorised_sender,
            create_pair_execute_message,
        )
        .unwrap_err();

        assert_eq!(result.to_string(), "Unauthorized")
    }

    #[test]
    fn create_pair_with_empty_route_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let result = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::CreatePair {
                base_denom: DENOM_UOSMO.to_string(),
                quote_denom: DENOM_STAKE.to_string(),
                route: vec![],
            },
        )
        .unwrap_err();

        assert_eq!(result.to_string(), "Error: Swap route must not be empty")
    }

    #[test]
    fn create_pair_with_invalid_route_should_fail() {
        let mut deps = calc_mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        let result = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::CreatePair {
                base_denom: DENOM_UOSMO.to_string(),
                quote_denom: DENOM_STAKE.to_string(),
                route: vec![2],
            },
        )
        .unwrap_err();

        assert_eq!(
            result.to_string(),
            "Generic error: denom uosmo not found in pool id 2"
        )
    }
}
