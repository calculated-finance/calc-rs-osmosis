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
    msg::MigrateMsg,
    state::config::{get_config, update_config},
    types::config::Config,
};
use cosmwasm_std::{DepsMut, Response, StdError};
use cw2::{get_contract_version, set_contract_version};

pub fn migrate_handler(deps: DepsMut, msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    if contract_version.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }

    #[allow(clippy::cmp_owned)]
    if contract_version.version > CONTRACT_VERSION.to_string() {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

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

    let config = get_config(deps.storage)?;

    update_config(
        deps.storage,
        Config {
            executors: msg.clone().executors,
            fee_collectors: msg.clone().fee_collectors,
            default_swap_fee_percent: msg.default_swap_fee_percent,
            weighted_scale_swap_fee_percent: msg.weighted_scale_swap_fee_percent,
            automation_fee_percent: msg.automation_fee_percent,
            default_page_limit: msg.default_page_limit,
            paused: msg.paused,
            risk_weighted_average_escrow_level: msg.risk_weighted_average_escrow_level,
            twap_period: msg.twap_period,
            default_slippage_tolerance: msg.default_slippage_tolerance,
            ..config
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "migrate")
        .add_attribute("msg", format!("{:#?}", msg)))
}
