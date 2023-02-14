use cosmwasm_std::{Addr, Deps, StdResult};

use crate::{msg::RoutersResponse, state::routers::get_routers_by_address};

pub fn get_routers_by_address_handler(
    deps: Deps,
    address: Addr,
) -> StdResult<RoutersResponse> {
    let routers = get_routers_by_address(deps.storage, address)?;

    Ok(RoutersResponse { routers })
}
