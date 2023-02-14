use cosmwasm_std::{Deps, StdResult};

use crate::{msg::FundResponse, state::funds::get_current_fund};

pub fn get_fund(deps: Deps) -> StdResult<FundResponse> {
    get_current_fund(deps).map(|fund_address| FundResponse {
        address: fund_address,
    })
}
