use std::cmp::min;

use crate::{
    state::config::{get_config, get_custom_fee},
    types::vault::Vault,
};
use cosmwasm_std::{BankMsg, Coin, Decimal, Deps, StdResult, Storage, SubMsg, Uint128};

use super::math::checked_mul;

pub fn get_fee_messages(
    deps: Deps,
    fee_amounts: Vec<Uint128>,
    denom: String,
) -> StdResult<Vec<SubMsg>> {
    let config = get_config(deps.storage)?;

    Ok(config
        .fee_collectors
        .iter()
        .flat_map(|fee_collector| {
            fee_amounts.iter().flat_map(|fee| {
                let fee_allocation = Coin::new(
                    checked_mul(*fee, fee_collector.allocation)
                        .expect("amount to be distributed should be valid")
                        .into(),
                    denom.clone(),
                );

                if fee_allocation.amount.is_zero() {
                    return None;
                }

                Some(SubMsg::new(BankMsg::Send {
                    to_address: fee_collector.address.to_string(),
                    amount: vec![fee_allocation],
                }))
            })
        })
        .collect::<Vec<SubMsg>>())
}

pub fn get_delegation_fee_rate(storage: &dyn Storage, vault: &Vault) -> StdResult<Decimal> {
    let config = get_config(storage)?;

    Ok(config.delegation_fee_percent.checked_mul(
        vault
            .destinations
            .iter()
            .filter(|destination| destination.msg.is_some())
            .map(|destination| destination.allocation)
            .sum(),
    )?)
}

pub fn get_swap_fee_rate(storage: &dyn Storage, vault: &Vault) -> StdResult<Decimal> {
    let config = get_config(storage)?;

    Ok(
        match (
            get_custom_fee(storage, vault.get_swap_denom()),
            get_custom_fee(storage, vault.target_denom.clone()),
        ) {
            (Some(swap_denom_fee_percent), Some(receive_denom_fee_percent)) => {
                min(swap_denom_fee_percent, receive_denom_fee_percent)
            }
            (Some(swap_denom_fee_percent), None) => swap_denom_fee_percent,
            (None, Some(receive_denom_fee_percent)) => receive_denom_fee_percent,
            (None, None) => config.swap_fee_percent,
        },
    )
}

pub fn get_dca_plus_performance_fee(vault: &Vault, current_price: Decimal) -> StdResult<Coin> {
    let dca_plus_config = vault
        .dca_plus_config
        .clone()
        .expect("DCA plus config for the vault");

    let dca_plus_total_value = dca_plus_config.total_deposit.amount - vault.swapped_amount.amount
        + vault.received_amount.amount * current_price;

    let standard_dca_total_value = dca_plus_config.total_deposit.amount
        - dca_plus_config.standard_dca_swapped_amount.amount
        + dca_plus_config.standard_dca_received_amount.amount * current_price;

    if standard_dca_total_value > dca_plus_total_value {
        return Ok(Coin {
            denom: vault.target_denom.clone(),
            amount: Uint128::zero(),
        });
    }

    let value_difference_in_terms_of_receive_denom =
        (dca_plus_total_value - standard_dca_total_value) * (Decimal::one() / current_price);

    let fee = value_difference_in_terms_of_receive_denom * Decimal::percent(20);

    Ok(Coin {
        denom: vault.target_denom.clone(),
        amount: min(fee, dca_plus_config.escrowed_balance.amount),
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::TEN,
        helpers::fees::get_dca_plus_performance_fee,
        types::{dca_plus_config::DcaPlusConfig, vault::Vault},
    };
    use cosmwasm_std::{Coin, Decimal, Uint128};
    use std::str::FromStr;

    fn get_vault(
        total_deposit: Uint128,
        swapped_amount: Uint128,
        standard_dca_swapped_amount: Uint128,
        received_amount: Uint128,
        standard_dca_received_amount: Uint128,
    ) -> Vault {
        let escrow_level = Decimal::percent(5);

        Vault {
            balance: Coin {
                denom: "swap_denom".to_string(),
                amount: total_deposit - swapped_amount,
            },
            swapped_amount: Coin {
                denom: "swap_denom".to_string(),
                amount: swapped_amount,
            },
            received_amount: Coin {
                denom: "receive_denom".to_string(),
                amount: received_amount,
            },
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(total_deposit.into(), "swap_denom".to_string()),
                standard_dca_swapped_amount: Coin::new(
                    standard_dca_swapped_amount.into(),
                    "swap_denom".to_string(),
                ),
                standard_dca_received_amount: Coin::new(
                    standard_dca_received_amount.into(),
                    "receive_denom".to_string(),
                ),
                escrowed_balance: Coin::new(
                    (received_amount * escrow_level).into(),
                    "denom".to_string(),
                ),
                model_id: 30,
                escrow_level,
            }),
            ..Vault::default()
        }
    }

    fn assert_fee_amount(
        total_deposit: Uint128,
        swapped_amount: Uint128,
        standard_dca_swapped_amount: Uint128,
        received_amount: Uint128,
        standard_dca_received_amount: Uint128,
        current_price: Decimal,
        expected_fee: Uint128,
    ) {
        let vault = get_vault(
            total_deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
        );

        let fee = get_dca_plus_performance_fee(&vault, current_price).unwrap();
        assert_eq!(fee.amount, expected_fee);
    }

    #[test]
    fn non_zero_fee_is_in_vault_receive_denom() {
        let vault = get_vault(TEN, TEN, TEN, TEN + TEN, TEN);

        let fee = get_dca_plus_performance_fee(&vault, Decimal::one()).unwrap();
        assert_eq!(fee.denom, vault.target_denom);
    }

    #[test]
    fn zero_fee_is_in_vault_receive_denom() {
        let vault = get_vault(TEN, TEN, TEN, TEN, TEN);

        let fee = get_dca_plus_performance_fee(&vault, Decimal::one()).unwrap();
        assert_eq!(fee.denom, vault.target_denom);
    }

    #[test]
    fn fee_is_zero_when_performance_is_even() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1.0").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            deposit,
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
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(900);
        let received_amount = Uint128::new(900);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.9").unwrap();
        let expected_fee = Uint128::new(2);

        assert_fee_amount(
            deposit,
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
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(900);
        let received_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.2").unwrap();
        let expected_fee = Uint128::new(50);

        assert_fee_amount(
            deposit,
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
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1100);
        let received_amount = Uint128::new(1000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("0.9").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            deposit,
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
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1100);
        let received_amount = Uint128::new(1100);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("2").unwrap();
        let expected_fee = Uint128::new(10);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }

    #[test]
    fn fee_is_equal_to_escrow_when_same_amount_swapped_and_more_received() {
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(1000);
        let received_amount = Uint128::new(2000);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1").unwrap();
        let expected_fee = Uint128::new(100);

        assert_fee_amount(
            deposit,
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
        let deposit = Uint128::new(2000);
        let swapped_amount = Uint128::new(900);
        let received_amount = Uint128::new(900);
        let standard_dca_swapped_amount = Uint128::new(1000);
        let standard_dca_received_amount = Uint128::new(1000);
        let current_price = Decimal::from_str("1.1").unwrap();
        let expected_fee = Uint128::new(0);

        assert_fee_amount(
            deposit,
            swapped_amount,
            standard_dca_swapped_amount,
            received_amount,
            standard_dca_received_amount,
            current_price,
            expected_fee,
        );
    }
}
