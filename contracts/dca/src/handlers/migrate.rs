use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    helpers::validation_helpers::{
        assert_dca_plus_escrow_level_is_less_than_100_percent,
        assert_fee_collector_addresses_are_valid, assert_fee_collector_allocations_add_up_to_one,
    },
    msg::MigrateMsg,
    state::config::{update_config, Config},
};
use cosmwasm_std::{DepsMut, Response};
use cw2::set_contract_version;

pub fn migrate_handler(deps: DepsMut, msg: MigrateMsg) -> Result<Response, ContractError> {
    deps.api.addr_validate(msg.admin.as_ref())?;

    assert_fee_collector_addresses_are_valid(deps.as_ref(), &msg.fee_collectors)?;
    assert_fee_collector_allocations_add_up_to_one(&msg.fee_collectors)?;
    assert_dca_plus_escrow_level_is_less_than_100_percent(msg.dca_plus_escrow_level)?;

    update_config(
        deps.storage,
        Config {
            admin: msg.admin.clone(),
            fee_collectors: msg.fee_collectors,
            swap_fee_percent: msg.swap_fee_percent,
            delegation_fee_percent: msg.delegation_fee_percent,
            page_limit: msg.page_limit,
            paused: msg.paused,
            dca_plus_escrow_level: msg.dca_plus_escrow_level,
        },
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}
