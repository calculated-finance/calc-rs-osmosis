use std::collections::VecDeque;

use cosmwasm_std::{Addr, Deps, DepsMut, StdResult};
use cw_storage_plus::Item;

pub const FUNDS: Item<VecDeque<Addr>> = Item::new("funds_v1");

pub fn initialise_funds(deps: DepsMut) -> StdResult<()> {
    let funds: VecDeque<Addr> = VecDeque::new();
    FUNDS.save(deps.storage, &funds)?;
    Ok(())
}

pub fn get_current_fund(deps: Deps) -> StdResult<Addr> {
    let funds = FUNDS.load(deps.storage)?;
    let current_fund = funds.front().expect("funds should not be empty");
    Ok(current_fund.clone())
}

pub fn save_fund(deps: DepsMut, fund_address: Addr) -> StdResult<Addr> {
    let mut funds = FUNDS.load(deps.storage)?;
    funds.push_front(fund_address.clone());
    FUNDS.save(deps.storage, &funds)?;
    Ok(fund_address)
}
