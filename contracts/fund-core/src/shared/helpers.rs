use crate::state::get_config;
use base::price_type::PriceType;
use cosmwasm_std::{Coin, Decimal, Deps, StdResult, Uint128};
use std::collections::HashMap;
use swap::msg::QueryMsg;

pub fn get_current_balance_values(
    deps: Deps,
    current_balances: &HashMap<String, Coin>,
) -> StdResult<HashMap<String, Uint128>> {
    let config = get_config(deps.storage)?;

    Ok(current_balances
        .values()
        .map(|asset| {
            if asset.denom == config.base_denom {
                return (asset.denom.clone(), asset.amount);
            }

            let price: Decimal = deps
                .querier
                .query_wasm_smart(
                    config.swapper.clone(),
                    &QueryMsg::GetPrice {
                        swap_amount: asset.clone(),
                        target_denom: config.base_denom.clone(),
                        price_type: PriceType::Belief,
                    },
                )
                .expect(&format!(
                    "price for swapping {:?} into {}",
                    asset, config.base_denom
                ));

            let asset_value_in_terms_of_base_denom = asset.amount * (Decimal::one() / price);

            (asset.denom.clone(), asset_value_in_terms_of_base_denom)
        })
        .collect::<HashMap<_, _>>())
}
