use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    helpers::validation::{
        assert_addresses_are_valid, assert_fee_collector_addresses_are_valid,
        assert_fee_collector_allocations_add_up_to_one, assert_fee_level_is_valid,
        assert_no_more_than_10_fee_collectors, assert_page_limit_is_valid,
        assert_risk_weighted_average_escrow_level_is_no_greater_than_100_percent,
        assert_slippage_tolerance_is_less_than_or_equal_to_one, assert_twap_period_is_valid,
    },
    msg::InstantiateMsg,
    state::config::update_config,
    types::config::Config,
};
use cosmwasm_std::{DepsMut, Response};
use cw2::set_contract_version;

pub fn instantiate_handler(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
    deps.api.addr_validate(msg.admin.as_ref())?;

    assert_fee_level_is_valid(&msg.default_swap_fee_percent)?;
    assert_fee_level_is_valid(&msg.weighted_scale_swap_fee_percent)?;
    assert_fee_level_is_valid(&msg.automation_fee_percent)?;
    assert_page_limit_is_valid(Some(msg.default_page_limit))?;
    assert_slippage_tolerance_is_less_than_or_equal_to_one(msg.default_slippage_tolerance)?;
    assert_twap_period_is_valid(msg.twap_period)?;
    assert_addresses_are_valid(deps.as_ref(), &msg.executors, "executor")?;
    assert_no_more_than_10_fee_collectors(&msg.fee_collectors)?;
    assert_fee_collector_addresses_are_valid(deps.as_ref(), &msg.fee_collectors)?;
    assert_fee_collector_allocations_add_up_to_one(&msg.fee_collectors)?;
    assert_risk_weighted_average_escrow_level_is_no_greater_than_100_percent(
        msg.risk_weighted_average_escrow_level,
    )?;

    update_config(
        deps.storage,
        Config {
            admin: msg.admin.clone(),
            executors: msg.executors,
            fee_collectors: msg.fee_collectors,
            default_swap_fee_percent: msg.default_swap_fee_percent,
            weighted_scale_swap_fee_percent: msg.weighted_scale_swap_fee_percent,
            automation_fee_percent: msg.automation_fee_percent,
            default_page_limit: msg.default_page_limit,
            paused: msg.paused,
            risk_weighted_average_escrow_level: msg.risk_weighted_average_escrow_level,
            twap_period: msg.twap_period,
            default_slippage_tolerance: msg.default_slippage_tolerance,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin))
}

#[cfg(test)]
mod instantiate_tests {
    use crate::contract::instantiate;
    use crate::msg::InstantiateMsg;
    use crate::types::fee_collector::FeeCollector;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, Addr, Decimal};
    use std::str::FromStr;

    pub const INVALID_ADDRESS: &str = "";
    pub const VALID_ADDRESS_ONE: &str = "osmo16q6jpx7ns0ugwghqay73uxd5aq30du3uqgxf0d";

    #[test]
    fn instantiate_with_valid_admin_address_should_succeed() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("", &vec![]);

        let instantiate_message = InstantiateMsg {
            admin: Addr::unchecked(VALID_ADDRESS_ONE),
            executors: vec![Addr::unchecked("executor")],
            fee_collectors: vec![FeeCollector {
                address: VALID_ADDRESS_ONE.to_string(),
                allocation: Decimal::from_str("1").unwrap(),
            }],
            default_swap_fee_percent: Decimal::from_str("0.015").unwrap(),
            weighted_scale_swap_fee_percent: Decimal::percent(1),
            automation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            default_page_limit: 30,
            paused: false,
            risk_weighted_average_escrow_level: Decimal::from_str("0.05").unwrap(),
            twap_period: 30,
            default_slippage_tolerance: Decimal::percent(2),
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
            executors: vec![Addr::unchecked("executor")],
            fee_collectors: vec![FeeCollector {
                address: VALID_ADDRESS_ONE.to_string(),
                allocation: Decimal::from_str("1").unwrap(),
            }],
            default_swap_fee_percent: Decimal::from_str("0.015").unwrap(),
            weighted_scale_swap_fee_percent: Decimal::percent(1),
            automation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            default_page_limit: 30,
            paused: false,
            risk_weighted_average_escrow_level: Decimal::from_str("0.05").unwrap(),
            twap_period: 30,
            default_slippage_tolerance: Decimal::percent(2),
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
            executors: vec![Addr::unchecked("executor")],
            fee_collectors: vec![FeeCollector {
                address: INVALID_ADDRESS.to_string(),
                allocation: Decimal::from_str("1").unwrap(),
            }],
            default_swap_fee_percent: Decimal::from_str("0.015").unwrap(),
            weighted_scale_swap_fee_percent: Decimal::percent(1),
            automation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            default_page_limit: 30,
            paused: false,
            risk_weighted_average_escrow_level: Decimal::from_str("0.05").unwrap(),
            twap_period: 30,
            default_slippage_tolerance: Decimal::percent(2),
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
            executors: vec![Addr::unchecked("executor")],
            fee_collectors: vec![],
            default_swap_fee_percent: Decimal::from_str("0.015").unwrap(),
            weighted_scale_swap_fee_percent: Decimal::percent(1),
            automation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            default_page_limit: 30,
            paused: false,
            risk_weighted_average_escrow_level: Decimal::from_str("0.05").unwrap(),
            twap_period: 30,
            default_slippage_tolerance: Decimal::percent(2),
        };

        let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

        assert_eq!(
            result.to_string(),
            "Error: fee collector allocations must add up to 1"
        )
    }
}
