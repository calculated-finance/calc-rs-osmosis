use base::ContractError;
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};

use crate::{
    state::config::{get_config, update_config, Config},
    validation_helpers::assert_sender_is_admin,
};

pub fn update_config_handler(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<Addr>,
    router_code_id: Option<u64>,
    fund_code_id: Option<u64>,
) -> Result<Response, ContractError> {
    assert_sender_is_admin(deps.storage, info.sender)?;

    let existing_config = get_config(deps.storage)?;

    let config = Config {
        admin: admin.unwrap_or(existing_config.admin),
        router_code_id: router_code_id.unwrap_or(existing_config.router_code_id),
        fund_code_id: fund_code_id.unwrap_or(existing_config.fund_code_id),
    };

    let config = update_config(deps.storage, config)?;

    deps.api.addr_validate(&config.admin.to_string())?;

    Ok(Response::new()
        .add_attribute("method", "update_config")
        .add_attribute("router_code_id", config.router_code_id.to_string()))
}
