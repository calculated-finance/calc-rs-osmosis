use crate::{state::swap_adjustments::get_swap_adjustment, types::vault::Vault};
use cosmwasm_std::{Coin, Deps, StdResult, Uint128};

pub fn get_swap_amount(vault: Vault, deps: &Deps) -> StdResult<Coin> {
    let initial_amount = match vault.low_funds() {
        true => vault.balance.amount,
        false => vault.swap_amount,
    };

    let adjusted_amount = vault
        .clone()
        .dca_plus_config
        .map_or(initial_amount, |dca_plus_config| {
            get_swap_adjustment(deps.storage, dca_plus_config.model_id)
                .map_or(initial_amount, |adjustment_coefficient| {
                    adjustment_coefficient * initial_amount
                })
        });

    Ok(Coin {
        denom: vault.get_swap_denom(),
        amount: adjusted_amount,
    })
}

pub fn has_sufficient_funds(vault: Vault, deps: &Deps) -> StdResult<bool> {
    get_swap_amount(vault, deps).map(|swap_amount| swap_amount.amount > Uint128::new(50000))
}
