use cosmwasm_std::{Addr, Deps, StdResult};

use crate::{msg::FundRoutersResponse, state::fund_routers::get_fund_routers_by_address};

pub fn get_fund_routers_by_address_handler(
    deps: Deps,
    address: Addr,
) -> StdResult<FundRoutersResponse> {
    let fund_routers = get_fund_routers_by_address(deps.storage, address)?;

    Ok(FundRoutersResponse { fund_routers })
}
