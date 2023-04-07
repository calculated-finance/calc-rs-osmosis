use crate::{msg::PairsResponse, state::pairs::PAIRS, types::pair::Pair};
use cosmwasm_std::{Deps, Order, StdResult};

pub fn get_pairs(deps: Deps) -> StdResult<PairsResponse> {
    let pairs = PAIRS
        .range(deps.storage, None, None, Order::Ascending)
        .flat_map(|result| result.map(|(_, pair)| pair))
        .collect::<Vec<Pair>>();

    Ok(PairsResponse { pairs })
}
