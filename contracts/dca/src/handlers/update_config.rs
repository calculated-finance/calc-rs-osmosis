use crate::{
    error::ContractError,
    helpers::validation::{
        assert_addresses_are_valid, assert_fee_collector_addresses_are_valid,
        assert_fee_collector_allocations_add_up_to_one, assert_fee_level_is_valid,
        assert_no_more_than_10_fee_collectors, assert_page_limit_is_valid,
        assert_risk_weighted_average_escrow_level_is_no_greater_than_100_percent,
        assert_sender_is_admin, assert_slippage_tolerance_is_less_than_or_equal_to_one,
        assert_twap_period_is_valid,
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
    default_swap_fee_percent: Option<Decimal>,
    weighted_scale_swap_fee_percent: Option<Decimal>,
    automation_fee_percent: Option<Decimal>,
    default_page_limit: Option<u16>,
    paused: Option<bool>,
    risk_weighted_average_escrow_level: Option<Decimal>,
    twap_period: Option<u64>,
    default_slippage_tolerance: Option<Decimal>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;
    let existing_config = get_config(deps.storage)?;

    let config = Config {
        admin: existing_config.admin,
        executors: executors.unwrap_or(existing_config.executors),
        fee_collectors: fee_collectors.unwrap_or(existing_config.fee_collectors),
        default_swap_fee_percent: default_swap_fee_percent
            .unwrap_or(existing_config.default_swap_fee_percent),
        weighted_scale_swap_fee_percent: weighted_scale_swap_fee_percent
            .unwrap_or(existing_config.weighted_scale_swap_fee_percent),
        automation_fee_percent: automation_fee_percent
            .unwrap_or(existing_config.automation_fee_percent),
        default_page_limit: default_page_limit.unwrap_or(existing_config.default_page_limit),
        paused: paused.unwrap_or(existing_config.paused),
        risk_weighted_average_escrow_level: risk_weighted_average_escrow_level
            .unwrap_or(existing_config.risk_weighted_average_escrow_level),
        twap_period: twap_period.unwrap_or(existing_config.twap_period),
        default_slippage_tolerance: default_slippage_tolerance
            .unwrap_or(existing_config.default_slippage_tolerance),
    };

    assert_fee_level_is_valid(&config.default_swap_fee_percent)?;
    assert_fee_level_is_valid(&config.weighted_scale_swap_fee_percent)?;
    assert_fee_level_is_valid(&config.automation_fee_percent)?;
    assert_page_limit_is_valid(Some(config.default_page_limit))?;
    assert_slippage_tolerance_is_less_than_or_equal_to_one(config.default_slippage_tolerance)?;
    assert_twap_period_is_valid(config.twap_period)?;
    assert_addresses_are_valid(deps.as_ref(), &config.executors, "executor")?;
    assert_no_more_than_10_fee_collectors(&config.fee_collectors)?;
    assert_fee_collector_addresses_are_valid(deps.as_ref(), &config.fee_collectors)?;
    assert_fee_collector_allocations_add_up_to_one(&config.fee_collectors)?;
    assert_risk_weighted_average_escrow_level_is_no_greater_than_100_percent(
        config.risk_weighted_average_escrow_level,
    )?;

    let config = update_config(deps.storage, config)?;

    Ok(Response::default()
        .add_attribute("method", "update_config")
        .add_attribute("config", format!("{:?}", config)))
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
            Some(Decimal::percent(2)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let config = get_config(deps.as_ref().storage).unwrap();

        assert_eq!(config.default_swap_fee_percent, Decimal::percent(2));
    }

    #[test]
    fn update_swap_fee_percent_more_than_5_percent_should_fail() {
        let mut deps = mock_dependencies();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), mock_env(), info.clone());

        let err = update_config_handler(
            deps.as_mut(),
            info,
            None,
            None,
            Some(Decimal::percent(15)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Error: fee level cannot be larger than 5%")
    }

    #[test]
    fn update_automation_fee_percent_with_valid_value_should_succeed() {
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
            Some(Decimal::percent(2)),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let config = get_config(deps.as_ref().storage).unwrap();

        assert_eq!(config.automation_fee_percent, Decimal::percent(2));
    }

    #[test]
    fn update_automation_fee_percent_more_than_5_percent_should_fail() {
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
            Some(Decimal::percent(15)),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Error: fee level cannot be larger than 5%")
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
            None,
            Some(Decimal::percent(19)),
            None,
            None,
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
            None,
            Some(Decimal::percent(150)),
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: risk_weighted_average_escrow_level cannot be greater than 100%"
        )
    }

    #[test]
    fn with_default_slippage_tolerance_more_than_100_percent_should_fail() {
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
            None,
            None,
            None,
            Some(Decimal::percent(150)),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: default slippage tolerance must be less than or equal to 1"
        )
    }

    #[test]
    fn with_more_than_10_fee_collectors_should_fail() {
        let mut deps = mock_dependencies();
        let info = mock_info(ADMIN, &[]);

        instantiate_contract(deps.as_mut(), mock_env(), info.clone());

        let err = update_config_handler(
            deps.as_mut(),
            info,
            None,
            Some(vec![
                FeeCollector {
                    address: "fee-collector".to_string(),
                    allocation: Decimal::percent(5),
                };
                20
            ]),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: no more than 10 fee collectors are allowed"
        )
    }

    #[test]
    fn with_page_limit_less_than_30_should_fail() {
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
            Some(10),
            None,
            None,
            None,
            None,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Error: limit cannot be less than 30.")
    }
}
