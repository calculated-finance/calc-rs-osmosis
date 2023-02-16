use crate::{
    state::paths::get_all_paths,
    types::{pair::Pair, path::Path},
};
use base::{pair::Pair as FinPair, price_type::PriceType};
use cosmwasm_std::{Coin, Decimal, Deps, QuerierWrapper, StdError, StdResult};
use fin_helpers::queries::query_price;

pub fn get_price(
    deps: Deps,
    swap_amount: &Coin,
    target_denom: &String,
    price_type: PriceType,
) -> StdResult<Decimal> {
    get_cheapest_swap_path(deps, swap_amount, target_denom, price_type).map(|path| path.price)
}

pub fn get_cheapest_swap_path(
    deps: Deps,
    swap_amount: &Coin,
    target_denom: &String,
    price_type: PriceType,
) -> StdResult<Path> {
    let possible_swap_paths =
        get_swap_paths_with_price(deps, swap_amount, &target_denom, price_type)?;

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
    price_type: PriceType,
) -> StdResult<Vec<Path>> {
    let mut paths = get_all_paths(deps.storage, &swap_amount.denom, target_denom)?
        .iter()
        .map(|pairs| Path {
            price: pairs
                .iter()
                .flat_map(|pair| {
                    get_price_for_pair(deps.querier, pair, swap_amount, price_type.clone())
                })
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
    price_type: PriceType,
) -> StdResult<Decimal> {
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
            price_type,
        ),
    }
}
