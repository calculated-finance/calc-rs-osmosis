use crate::{msg::PairsResponse, state::pairs::PAIRS};
use base::pair::Pair;
use cosmwasm_std::{Deps, Order, StdResult};

pub fn get_pairs(deps: Deps) -> StdResult<PairsResponse> {
    let all_pairs_on_heap: StdResult<Vec<_>> = PAIRS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();

    let pairs: Vec<Pair> = all_pairs_on_heap
        .unwrap()
        .iter()
        .map(|p| p.1.clone())
        .collect();

    Ok(PairsResponse { pairs })
}
