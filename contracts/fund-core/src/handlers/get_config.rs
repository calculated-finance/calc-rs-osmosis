use cosmwasm_std::{Deps, StdResult};

use crate::{msg::ConfigResponse, state::get_config};

pub fn get_config_handler(deps: Deps) -> StdResult<ConfigResponse> {
    get_config(deps.storage).map(|config| ConfigResponse { config })
}
