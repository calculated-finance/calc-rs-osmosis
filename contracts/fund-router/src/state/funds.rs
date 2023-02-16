use std::collections::VecDeque;

use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::Item;

pub const FUNDS: Item<VecDeque<Addr>> = Item::new("funds_v1");

pub fn initialise_funds(storage: &mut dyn Storage) -> StdResult<()> {
    let funds: VecDeque<Addr> = VecDeque::new();
    FUNDS.save(storage, &funds)?;
    Ok(())
}

pub fn get_current_fund(storage: &dyn Storage) -> StdResult<Option<Addr>> {
    let mut funds = FUNDS.load(storage)?;
    let current_fund = funds.pop_front();
    Ok(current_fund.clone())
}

pub fn save_fund(storage: &mut dyn Storage, fund_address: Addr) -> StdResult<Addr> {
    let mut funds = FUNDS.load(storage)?;
    funds.push_front(fund_address.clone());
    FUNDS.save(storage, &funds)?;
    Ok(fund_address)
}
