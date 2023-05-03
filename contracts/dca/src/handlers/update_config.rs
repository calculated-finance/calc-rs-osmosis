use crate::{
    error::ContractError,
    helpers::validation::{
        assert_addresses_are_valid, assert_fee_collector_addresses_are_valid,
        assert_fee_collector_allocations_add_up_to_one,
        assert_risk_weighted_average_escrow_level_is_less_than_100_percent, assert_sender_is_admin,
    },
    state::config::{get_config, update_config},
    types::{config::Config, fee_collector::FeeCollector},
};
use cosmwasm_std::{Addr, Decimal, DepsMut, MessageInfo, Response};

pub fn update_config_handler(
    deps: DepsMut,
    info: MessageInfo,
    executors: Option<Vec<Addr>>,
    fee_collectors: Option<Vec<FeeCollector>>,
    swap_fee_percent: Option<Decimal>,
    delegation_fee_percent: Option<Decimal>,
    page_limit: Option<u16>,
    paused: Option<bool>,
    risk_weighted_average_escrow_level: Option<Decimal>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;
    let existing_config = get_config(deps.storage)?;

    let config = Config {
        admin: existing_config.admin,
        executors: executors.unwrap_or(existing_config.executors),
        fee_collectors: fee_collectors.unwrap_or(existing_config.fee_collectors),
        swap_fee_percent: swap_fee_percent.unwrap_or(existing_config.swap_fee_percent),
        delegation_fee_percent: delegation_fee_percent
            .unwrap_or(existing_config.delegation_fee_percent),
        page_limit: page_limit.unwrap_or(existing_config.page_limit),
        paused: paused.unwrap_or(existing_config.paused),
        risk_weighted_average_escrow_level: risk_weighted_average_escrow_level
            .unwrap_or(existing_config.risk_weighted_average_escrow_level),
    };

    assert_addresses_are_valid(deps.as_ref(), &config.executors, "executor")?;
    assert_fee_collector_addresses_are_valid(deps.as_ref(), &config.fee_collectors)?;
    assert_fee_collector_allocations_add_up_to_one(&config.fee_collectors)?;
    assert_risk_weighted_average_escrow_level_is_less_than_100_percent(
        config.risk_weighted_average_escrow_level,
    )?;

    let config = update_config(deps.storage, config)?;

    Ok(Response::default()
        .add_attribute("method", "update_config")
        .add_attribute("swap_fee_percent", config.swap_fee_percent.to_string())
        .add_attribute("fee_collector", format!("{:?}", config.fee_collectors))
        .add_attribute(
            "delegation_fee_percent",
            config.delegation_fee_percent.to_string(),
        )
        .add_attribute("paused", config.paused.to_string()))
}

#[cfg(test)]
mod update_config_tests {
    use super::*;
    use crate::{
        state::config::get_config,
        tests::{helpers::instantiate_contract, mocks::ADMIN},
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Decimal,
    };
    use std::str::FromStr;

    #[test]
    fn update_executors_with_no_value_should_not_change_value() {
        let mut deps = mock_dependencies();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), mock_env(), info.clone());

        let config_before_update = get_config(deps.as_ref().storage).unwrap();

        update_config_handler(
            deps.as_mut(),
            info,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let config_after_update = get_config(deps.as_ref().storage).unwrap();

        assert_eq!(
            config_after_update.executors,
            config_before_update.executors
        );
    }

    #[test]
    fn update_executors_with_valid_value_should_succeed() {
        let mut deps = mock_dependencies();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), mock_env(), info.clone());

        let executors = Some(vec![
            Addr::unchecked("executor-1"),
            Addr::unchecked("executor-2"),
        ]);

        update_config_handler(
            deps.as_mut(),
            info,
            executors.clone(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let config = get_config(deps.as_ref().storage).unwrap();

        assert_eq!(config.executors, executors.unwrap());
    }

    #[test]
    fn update_swap_fee_percent_with_valid_value_should_succeed() {
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

        update_config_handler(
            deps.as_mut(),
            info,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

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
            None,
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
            None,
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
    fn update_risk_weighted_average_escrow_level_with_valid_value_should_succeed() {
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
            None,
            Some(Decimal::percent(19)),
        )
        .unwrap();

        let config = get_config(deps.as_ref().storage).unwrap();

        assert_eq!(
            config.risk_weighted_average_escrow_level,
            Decimal::percent(19)
        );
    }

    #[test]
    fn update_risk_weighted_average_escrow_level_more_than_100_percent_should_fail() {
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
            None,
            Some(Decimal::percent(150)),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: risk_weighted_average_escrow_level cannot be greater than 100%"
        )
    }
}
