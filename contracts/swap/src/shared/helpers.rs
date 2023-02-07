use crate::{
    state::paths::get_all_paths,
    types::{pair::Pair, path::Path},
};
use base::pair::Pair as FinPair;
use cosmwasm_std::{Coin, Decimal256, Deps, QuerierWrapper, StdError, StdResult};
use fin_helpers::queries::query_price;

pub fn get_cheapest_swap_path(
    deps: Deps,
    swap_amount: &Coin,
    target_denom: &String,
) -> StdResult<Path> {
    let possible_swap_paths = get_swap_paths_with_price(deps, swap_amount, &target_denom)?;

    if possible_swap_paths.is_empty() {
        return Err(StdError::generic_err(format!(
            "no path found between {} and {}",
            swap_amount.denom, target_denom
        )));
    }

    Ok(possible_swap_paths.first().expect("cheapest path").clone())
}

pub fn get_swap_paths_with_price(
    deps: Deps,
    swap_amount: &Coin,
    target_denom: &String,
) -> StdResult<Vec<Path>> {
    let mut paths = get_all_paths(deps.storage, &swap_amount.denom, target_denom)?
        .iter()
        .map(|pairs| Path {
            price: pairs
                .iter()
                .flat_map(|pair| get_price_for_pair(deps.querier, pair, swap_amount))
                .reduce(|acc, price| acc * price)
                .expect("total price of the swap"),
            pairs: pairs.clone(),
        })
        .collect::<Vec<Path>>();

    paths.sort_by_key(|path| path.price);

    Ok(paths)
}

pub fn get_price_for_pair(
    querier: QuerierWrapper,
    pair: &Pair,
    swap_amount: &Coin,
) -> StdResult<Decimal256> {
    match pair.clone() {
        Pair::Fin {
            address,
            base_denom,
            quote_denom,
        } => query_price(
            querier,
            FinPair {
                address,
                base_denom,
                quote_denom,
            },
            swap_amount,
        ),
    }
}
