use crate::{
    error::ContractError,
    helpers::validation_helpers::{
        assert_dca_plus_escrow_level_is_less_than_100_percent,
        assert_fee_collector_addresses_are_valid, assert_fee_collector_allocations_add_up_to_one,
        assert_sender_is_admin,
    },
    state::config::{get_config, update_config, Config, FeeCollector},
};
use cosmwasm_std::{Addr, Decimal, DepsMut, MessageInfo, Response};

pub fn update_config_handler(
    deps: DepsMut,
    info: MessageInfo,
    fee_collectors: Option<Vec<FeeCollector>>,
    swap_fee_percent: Option<Decimal>,
    delegation_fee_percent: Option<Decimal>,
    staking_router_address: Option<Addr>,
    page_limit: Option<u16>,
    paused: Option<bool>,
    dca_plus_escrow_level: Option<Decimal>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;
    let existing_config = get_config(deps.storage)?;

    let config = Config {
        admin: existing_config.admin,
        fee_collectors: fee_collectors.unwrap_or(existing_config.fee_collectors),
        swap_fee_percent: swap_fee_percent.unwrap_or(existing_config.swap_fee_percent),
        delegation_fee_percent: delegation_fee_percent
            .unwrap_or(existing_config.delegation_fee_percent),
        staking_router_address: deps.api.addr_validate(
            &staking_router_address
                .unwrap_or(existing_config.staking_router_address)
                .to_string(),
        )?,
        page_limit: page_limit.unwrap_or(existing_config.page_limit),
        paused: paused.unwrap_or(existing_config.paused),
        dca_plus_escrow_level: dca_plus_escrow_level
            .unwrap_or(existing_config.dca_plus_escrow_level),
    };

    assert_fee_collector_addresses_are_valid(deps.as_ref(), &config.fee_collectors)?;
    assert_fee_collector_allocations_add_up_to_one(&config.fee_collectors)?;
    assert_dca_plus_escrow_level_is_less_than_100_percent(config.dca_plus_escrow_level)?;

    let config = update_config(deps.storage, config)?;

    Ok(Response::default()
        .add_attribute("method", "update_config")
        .add_attribute("swap_fee_percent", config.swap_fee_percent.to_string())
        .add_attribute("fee_collector", format!("{:?}", config.fee_collectors))
        .add_attribute(
            "delegation_fee_percent",
            config.delegation_fee_percent.to_string(),
        )
        .add_attribute(
            "staking_router_address",
            config.staking_router_address.to_string(),
        )
        .add_attribute("paused", config.paused.to_string()))
}
