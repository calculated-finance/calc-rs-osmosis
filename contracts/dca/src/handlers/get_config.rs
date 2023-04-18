use crate::{msg::ConfigResponse, state::config::get_config};
use cosmwasm_std::{Deps, StdResult};

pub fn get_config_handler(deps: Deps) -> StdResult<ConfigResponse> {
    Ok(ConfigResponse {
        config: get_config(deps.storage)?,
    })
}
