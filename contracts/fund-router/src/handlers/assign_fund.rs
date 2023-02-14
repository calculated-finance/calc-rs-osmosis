use base::ContractError;
use cosmwasm_std::{Addr, DepsMut, Response};

use crate::state::funds::save_fund;

pub fn assign_fund(deps: DepsMut, fund_address: Addr) -> Result<Response, ContractError> {
    deps.api.addr_validate(fund_address.as_str())?;

    save_fund(deps, fund_address.clone())?;
    Ok(Response::new()
        .add_attribute("method", "assign_fund")
        .add_attribute("fund_address", fund_address.to_string()))
}
