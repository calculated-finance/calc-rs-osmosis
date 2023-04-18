use crate::state::config::get_custom_fees;
use cosmwasm_std::{Decimal, Deps, StdResult};

pub fn get_custom_swap_fees_handler(deps: Deps) -> StdResult<Vec<(String, Decimal)>> {
    get_custom_fees(deps.storage)
}
