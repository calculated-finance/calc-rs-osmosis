use crate::{state::swap_adjustments::get_swap_adjustment, types::vault::Vault};
use base::{helpers::time_helpers::get_total_execution_duration, triggers::trigger::TimeInterval};
use cosmwasm_std::{Coin, Decimal, Deps, StdResult, Timestamp, Uint128};
use std::cmp::min;

pub fn get_swap_amount(deps: &Deps, vault: Vault) -> StdResult<Coin> {
    let initial_amount = match vault.has_low_funds() {
        true => vault.balance.amount,
        false => vault.swap_amount,
    };

    let adjusted_amount = vault
        .clone()
        .dca_plus_config
        .map_or(initial_amount, |dca_plus_config| {
            get_swap_adjustment(
                deps.storage,
                vault.get_position_type(),
                dca_plus_config.model_id,
            )
            .map_or(initial_amount, |adjustment_coefficient| {
                adjustment_coefficient * initial_amount
            })
        });

    Ok(Coin {
        denom: vault.get_swap_denom(),
        amount: min(adjusted_amount, vault.balance.amount),
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

pub fn get_dca_plus_fee(vault: &Vault, current_price: Decimal) -> StdResult<Coin> {
    let dca_plus_config = vault
        .dca_plus_config
        .clone()
        .expect("Only DCA plus vaults should try to get performance");

    let dca_plus_total_value = vault.balance.amount + vault.received_amount.amount * current_price;

    let standard_dca_remaining_balance =
        vault.get_total_deposit_amount() - dca_plus_config.standard_dca_swapped_amount;

    let standard_dca_total_value = standard_dca_remaining_balance
        + dca_plus_config.standard_dca_received_amount * current_price;

    if standard_dca_total_value > dca_plus_total_value {
        return Ok(Coin {
            denom: vault.get_swap_denom(),
            amount: Uint128::zero(),
        });
    }

    let value_difference_in_terms_of_receive_denom =
        (dca_plus_total_value - standard_dca_total_value) * (Decimal::one() / current_price);

    let fee = value_difference_in_terms_of_receive_denom * Decimal::percent(20);

    Ok(Coin {
        denom: vault.get_swap_denom(),
        amount: min(fee, dca_plus_config.escrowed_balance),
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        helpers::vault_helpers::get_dca_plus_fee,
        types::{dca_plus_config::DCAPlusConfig, vault::Vault},
    };
    use base::{pair::Pair, triggers::trigger::TimeInterval, vaults::vault::VaultStatus};
    use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
    use std::str::FromStr;

    fn assert_fee_amount(
        remaining_balance: Uint128,
        swapped_amount: Uint128,
        standard_dca_swapped_amount: Uint128,
        received_amount: Uint128,
        standard_dca_received_amount: Uint128,
        current_price: Decimal,
        expected_fee: Uint128,
    ) {
        let escrow_level = Decimal::percent(5);

        let vault = Vault {
            balance: Coin {
                denom: "denom".to_string(),
                amount: remaining_balance,
            },
            swapped_amount: Coin {
                denom: "swap_denom".to_string(),
                amount: swapped_amount,
            },
            received_amount: Coin {
                denom: "receive_denom".to_string(),
                amount: received_amount,
            },
            dca_plus_config: Some(DCAPlusConfig {
                standard_dca_swapped_amount,
                standard_dca_received_amount,
                escrowed_balance: received_amount * escrow_level,
                model_id: 30,
                escrow_level,
            }),
            id: Uint128::one(),
            created_at: Timestamp::from_seconds(10000000),
            owner: Addr::unchecked("owner"),
            label: None,
            destinations: vec![],
            status: VaultStatus::Active,
            pair: Pair {
                address: Addr::unchecked("pair"),
                base_denom: "receive_denom".to_string(),
                quote_denom: "swap_denom".to_string(),
            },
            swap_amount: swapped_amount / Uint128::new(2),
            slippage_tolerance: None,
            minimum_receive_amount: None,
            time_interval: TimeInterval::Daily,
            started_at: None,
            trigger: None,
        };

        let fee = get_dca_plus_fee(&vault, current_price).unwrap();

        assert_eq!(fee.amount, expected_fee);
    }

    #[test]
    fn fee_is_zero_when_performance_is_even() {
        let remaining_balance = Uint128::new(1000);
        let swapped_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1.0").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            remaining_balance,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_above_zero_when_less_swapped_and_price_dropped() {
        let remaining_balance = Uint128::new(1000);
        let swapped_amount = Uint128::new(900);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.9").unwrap();
        let expected_fee = Uint128::new(22);

        assert_fee_amount(
            remaining_balance,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_equal_to_escrow_when_less_swapped_and_price_dropped_significantly() {
        let remaining_balance = Uint128::new(1000);
        let swapped_amount = Uint128::new(900);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.2").unwrap();
        let expected_fee = Uint128::new(50);

        assert_fee_amount(
            remaining_balance,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_zero_when_more_swapped_and_price_dropped() {
        let remaining_balance = Uint128::new(1000);
        let swapped_amount = Uint128::new(1100);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.9").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            remaining_balance,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_above_zero_when_more_swapped_and_price_increased() {
        let remaining_balance = Uint128::new(1000);
        let swapped_amount = Uint128::new(1100);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1100);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("2").unwrap();
        let expected_fee = Uint128::new(10);

        assert_fee_amount(
            remaining_balance,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_equal_to_escrow_when_more_received() {
        let remaining_balance = Uint128::new(1000);
        let swapped_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(2000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1").unwrap();
        let expected_fee = Uint128::new(100);

        assert_fee_amount(
            remaining_balance,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_zero_when_less_swapped_and_price_increased() {
        let remaining_balance = Uint128::new(1100);
        let swapped_amount = Uint128::new(900);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(900);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1.1").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            remaining_balance,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }
}
