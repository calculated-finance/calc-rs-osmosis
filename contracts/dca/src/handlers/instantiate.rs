use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    helpers::validation::{
        assert_addresses_are_valid, assert_fee_collector_addresses_are_valid,
        assert_fee_collector_allocations_add_up_to_one,
        assert_risk_weighted_average_escrow_level_is_less_than_100_percent,
    },
    msg::InstantiateMsg,
    state::config::update_config,
    types::config::Config,
};
use cosmwasm_std::{DepsMut, Response};
use cw2::set_contract_version;

pub fn instantiate_handler(deps: DepsMut, msg: InstantiateMsg) -> Result<Response, ContractError> {
    deps.api.addr_validate(msg.admin.as_ref())?;

    assert_addresses_are_valid(deps.as_ref(), &msg.executors, "executor")?;
    assert_fee_collector_addresses_are_valid(deps.as_ref(), &msg.fee_collectors)?;
    assert_fee_collector_allocations_add_up_to_one(&msg.fee_collectors)?;
    assert_risk_weighted_average_escrow_level_is_less_than_100_percent(
        msg.risk_weighted_average_escrow_level,
    )?;

    update_config(
        deps.storage,
        Config {
            admin: msg.admin.clone(),
            executors: msg.executors,
            fee_collectors: msg.fee_collectors,
            swap_fee_percent: msg.swap_fee_percent,
            delegation_fee_percent: msg.delegation_fee_percent,
            page_limit: msg.page_limit,
            paused: msg.paused,
            risk_weighted_average_escrow_level: msg.risk_weighted_average_escrow_level,
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
            swap_fee_percent: Decimal::from_str("0.015").unwrap(),
            delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            page_limit: 1000,
            paused: false,
            risk_weighted_average_escrow_level: Decimal::from_str("0.05").unwrap(),
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
            swap_fee_percent: Decimal::from_str("0.015").unwrap(),
            delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            page_limit: 1000,
            paused: false,
            risk_weighted_average_escrow_level: Decimal::from_str("0.05").unwrap(),
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
            swap_fee_percent: Decimal::from_str("0.015").unwrap(),
            delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            page_limit: 1000,
            paused: false,
            risk_weighted_average_escrow_level: Decimal::from_str("0.05").unwrap(),
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
            swap_fee_percent: Decimal::from_str("0.015").unwrap(),
            delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
            page_limit: 1000,
            paused: false,
            risk_weighted_average_escrow_level: Decimal::from_str("0.05").unwrap(),
        };

        let result = instantiate(deps.as_mut(), env, info, instantiate_message).unwrap_err();

        assert_eq!(
            result.to_string(),
            "Error: fee collector allocations must add up to 1"
        )
    }
}
