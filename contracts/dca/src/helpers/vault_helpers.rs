use crate::{state::swap_adjustments::get_swap_adjustment, types::vault::Vault};
use base::{helpers::time_helpers::get_total_execution_duration, triggers::trigger::TimeInterval};
use cosmwasm_std::{Coin, Deps, StdResult, Timestamp, Uint128};

pub fn get_swap_amount(deps: &Deps, vault: Vault) -> StdResult<Coin> {
    let initial_amount = match vault.low_funds() {
        true => vault.balance.amount,
        false => vault.swap_amount,
    };

    let adjusted_amount = vault
        .clone()
        .dca_plus_config
        .map_or(initial_amount, |dca_plus_config| {
            get_swap_adjustment(
                deps.storage,
                dca_plus_config.direction,
                dca_plus_config.model_id,
            )
            .map_or(initial_amount, |adjustment_coefficient| {
                adjustment_coefficient * initial_amount
            })
        });

    Ok(Coin {
        denom: vault.get_swap_denom(),
        amount: adjusted_amount,
    })
}

pub fn has_sufficient_funds(deps: &Deps, vault: Vault) -> StdResult<bool> {
    get_swap_amount(deps, vault).map(|swap_amount| swap_amount.amount > Uint128::new(50000))
}

pub fn get_dca_plus_model_id(
    block_time: &Timestamp,
    balance: &Coin,
    swap_amount: &Uint128,
    time_interval: &TimeInterval,
) -> u8 {
    let execution_duration = get_total_execution_duration(
        *block_time,
        (balance
            .amount
            .checked_div(*swap_amount)
            .expect("deposit divided by swap amount should be larger than 0"))
        .into(),
        &time_interval,
    );

    match execution_duration.num_days() {
        0..=32 => 30,
        33..=38 => 35,
        39..=44 => 40,
        45..=51 => 45,
        52..=57 => 50,
        58..=65 => 55,
        66..=77 => 60,
        78..=96 => 70,
        97..=123 => 80,
        _ => 90,
    }
}
