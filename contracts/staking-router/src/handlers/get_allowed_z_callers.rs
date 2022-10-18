use cosmwasm_std::{Addr, Deps, StdResult};

use crate::state::CONFIG;

pub fn get_allowed_z_callers(deps: Deps) -> StdResult<Vec<Addr>> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.allowed_z_callers)
}
