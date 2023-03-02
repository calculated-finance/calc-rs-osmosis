use super::dca_plus_config::DCAPlusConfig;
use base::{
    pair::Pair,
    triggers::trigger::{TimeInterval, TriggerConfiguration},
    vaults::vault::{Destination, VaultStatus},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal256, StdError, StdResult, Timestamp, Uint128};
use fin_helpers::position_type::PositionType;
use kujira::precision::{Precise, Precision};
use std::str::FromStr;

#[cw_serde]
pub struct Vault {
    pub id: Uint128,
    pub created_at: Timestamp,
    pub owner: Addr,
    pub label: Option<String>,
    pub destinations: Vec<Destination>,
    pub status: VaultStatus,
    pub balance: Coin,
    pub pair: Pair,
    pub swap_amount: Uint128,
    pub slippage_tolerance: Option<Decimal256>,
    pub minimum_receive_amount: Option<Uint128>,
    pub time_interval: TimeInterval,
    pub started_at: Option<Timestamp>,
    pub swapped_amount: Coin,
    pub received_amount: Coin,
    pub trigger: Option<TriggerConfiguration>,
    pub dca_plus_config: Option<DCAPlusConfig>,
}

impl Vault {
    pub fn get_position_type(&self) -> PositionType {
        match self.balance.denom == self.pair.quote_denom {
            true => PositionType::Enter,
            false => PositionType::Exit,
        }
    }

    pub fn get_swap_denom(&self) -> String {
        self.balance.denom.clone()
    }

    pub fn get_receive_denom(&self) -> String {
        if self.balance.denom == self.pair.quote_denom {
            return self.pair.base_denom.clone();
        }
        self.pair.quote_denom.clone()
    }

    pub fn get_total_deposit_amount(&self) -> Uint128 {
        self.balance.amount + self.swapped_amount.amount
    }

    pub fn get_target_price(
        &self,
        target_receive_amount: Uint128,
        decimal_delta: i8,
        precision: Precision,
    ) -> StdResult<Decimal256> {
        if decimal_delta < 0 {
            return Err(StdError::GenericErr {
                msg: "Negative decimal deltas are not supported".to_string(),
            });
        }

        let exact_target_price = match self.get_position_type() {
            PositionType::Enter => Decimal256::from_ratio(self.swap_amount, target_receive_amount),
            PositionType::Exit => Decimal256::from_ratio(target_receive_amount, self.swap_amount),
        };

        if decimal_delta == 0 {
            return Ok(exact_target_price.round(&precision));
        }

        let adjustment =
            Decimal256::from_str(&10u128.pow(decimal_delta.abs() as u32).to_string()).unwrap();

        let rounded_price = exact_target_price
            .checked_mul(adjustment)
            .unwrap()
            .round(&precision);

        Ok(rounded_price.checked_div(adjustment).unwrap())
    }

    pub fn price_threshold_exceeded(&self, price: Decimal256) -> bool {
        if let Some(minimum_receive_amount) = self.minimum_receive_amount {
            let target_swap_amount_as_decimal =
                Decimal256::from_ratio(self.swap_amount, Uint128::one());

            let receive_amount_at_price = match self.get_position_type() {
                PositionType::Enter => target_swap_amount_as_decimal
                    .checked_div(price)
                    .expect("current fin price should be > 0.0"),
                PositionType::Exit => target_swap_amount_as_decimal
                    .checked_mul(price)
                    .expect("expected receive amount should be valid"),
            };

            let minimum_receive_amount =
                Decimal256::from_ratio(minimum_receive_amount, Uint128::one());

            return receive_amount_at_price < minimum_receive_amount;
        }

        false
    }

    pub fn has_low_funds(&self) -> bool {
        self.balance.amount < self.swap_amount
    }

    pub fn is_active(&self) -> bool {
        self.status == VaultStatus::Active
    }

    pub fn is_scheduled(&self) -> bool {
        self.status == VaultStatus::Scheduled
    }

    pub fn is_inactive(&self) -> bool {
        self.status == VaultStatus::Inactive
    }
}

#[cfg(test)]
mod has_sufficient_funds_tests {
    use crate::{
        helpers::vault_helpers::has_sufficient_funds, state::vaults::save_vault,
        types::vault_builder::VaultBuilder,
    };

    use super::*;
    use cosmwasm_std::{
        coin,
        testing::{mock_dependencies, mock_env},
    };

    #[test]
    fn should_return_false_when_vault_has_insufficient_swap_amount() {
        let mut deps = mock_dependencies();
        let vault_builder = vault_with(100000, Uint128::new(50000));
        let vault = save_vault(deps.as_mut().storage, vault_builder).unwrap();
        assert!(!has_sufficient_funds(&deps.as_ref(), &mock_env(), vault).unwrap());
    }

    #[test]
    fn should_return_false_when_vault_has_insufficient_balance() {
        let mut deps = mock_dependencies();
        let vault_builder = vault_with(50000, Uint128::new(50001));
        let vault = save_vault(deps.as_mut().storage, vault_builder).unwrap();
        assert!(!has_sufficient_funds(&deps.as_ref(), &mock_env(), vault).unwrap());
    }

    #[test]
    fn should_return_true_when_vault_has_sufficient_swap_amount() {
        let mut deps = mock_dependencies();
        let vault_builder = vault_with(100000, Uint128::new(50001));
        let vault = save_vault(deps.as_mut().storage, vault_builder).unwrap();
        assert!(has_sufficient_funds(&deps.as_ref(), &mock_env(), vault).unwrap());
    }

    #[test]
    fn should_return_true_when_vault_has_sufficient_balance() {
        let mut deps = mock_dependencies();
        let vault_builder = vault_with(50001, Uint128::new(50002));
        let vault = save_vault(deps.as_mut().storage, vault_builder).unwrap();
        assert!(has_sufficient_funds(&deps.as_ref(), &mock_env(), vault).unwrap());
    }

    fn vault_with(balance: u128, swap_amount: Uint128) -> VaultBuilder {
        VaultBuilder::new(
            Timestamp::from_seconds(0),
            Addr::unchecked("owner"),
            None,
            vec![],
            VaultStatus::Active,
            coin(balance, "quote"),
            Pair {
                address: Addr::unchecked("pair"),
                base_denom: "base".to_string(),
                quote_denom: "quote".to_string(),
            },
            swap_amount,
            None,
            None,
            None,
            TimeInterval::Daily,
            None,
            Coin {
                denom: "quote".to_string(),
                amount: Uint128::new(0),
            },
            Coin {
                denom: "base".to_string(),
                amount: Uint128::new(0),
            },
            None,
        )
    }
}

#[cfg(test)]
mod price_threshold_exceeded_tests {
    use super::*;
    use cosmwasm_std::coin;
    use std::str::FromStr;

    #[test]
    fn should_not_be_exceeded_for_buying_on_fin_when_price_is_below_threshold() {
        let vault = vault_with(Uint128::new(100), Uint128::new(50), PositionType::Enter);

        assert_eq!(
            vault.price_threshold_exceeded(Decimal256::from_str("1.9").unwrap()),
            false
        );
    }

    #[test]
    fn should_not_be_exceeded_for_buying_on_fin_when_price_equals_threshold() {
        let vault = vault_with(Uint128::new(100), Uint128::new(50), PositionType::Enter);

        assert_eq!(
            vault.price_threshold_exceeded(Decimal256::from_str("2.0").unwrap()),
            false
        );
    }

    #[test]
    fn should_be_exceeded_for_buying_on_fin_when_price_is_above_threshold() {
        let vault = vault_with(Uint128::new(100), Uint128::new(50), PositionType::Enter);

        assert_eq!(
            vault.price_threshold_exceeded(Decimal256::from_str("2.1").unwrap()),
            true
        );
    }

    #[test]
    fn should_not_be_exceeded_for_selling_on_fin_when_price_is_above_threshold() {
        let vault = vault_with(Uint128::new(100), Uint128::new(50), PositionType::Exit);

        assert_eq!(
            vault.price_threshold_exceeded(Decimal256::from_str("0.51").unwrap()),
            false
        );
    }

    #[test]
    fn should_not_be_exceeded_for_selling_on_fin_when_price_equals_threshold() {
        let vault = vault_with(Uint128::new(100), Uint128::new(50), PositionType::Exit);

        assert_eq!(
            vault.price_threshold_exceeded(Decimal256::from_str("0.50").unwrap()),
            false
        );
    }

    #[test]
    fn should_be_exceeded_for_selling_on_fin_when_price_is_below_threshold() {
        let vault = vault_with(Uint128::new(100), Uint128::new(50), PositionType::Exit);

        assert_eq!(
            vault.price_threshold_exceeded(Decimal256::from_str("0.49").unwrap()),
            true
        );
    }

    fn vault_with(
        swap_amount: Uint128,
        minimum_receive_amount: Uint128,
        position_type: PositionType,
    ) -> Vault {
        Vault {
            id: Uint128::new(1),
            created_at: Timestamp::from_seconds(0),
            owner: Addr::unchecked("owner"),
            label: None,
            destinations: vec![],
            status: VaultStatus::Active,
            balance: coin(
                1000,
                match position_type {
                    PositionType::Enter => "quote",
                    PositionType::Exit => "base",
                },
            ),
            pair: Pair {
                address: Addr::unchecked("pair"),
                base_denom: "base".to_string(),
                quote_denom: "quote".to_string(),
            },
            swap_amount,
            slippage_tolerance: None,
            minimum_receive_amount: Some(minimum_receive_amount),
            time_interval: TimeInterval::Daily,
            started_at: None,
            swapped_amount: coin(
                0,
                match position_type {
                    PositionType::Enter => "quote",
                    PositionType::Exit => "base",
                },
            ),
            received_amount: coin(
                0,
                match position_type {
                    PositionType::Enter => "base",
                    PositionType::Exit => "quote",
                },
            ),
            trigger: None,
            dca_plus_config: None,
        }
    }
}

#[cfg(test)]
mod get_target_price_tests {
    use super::*;
    use cosmwasm_std::coin;

    #[test]
    fn should_be_correct_when_buying_on_fin() {
        let vault = vault_with(Uint128::new(100), PositionType::Enter);
        assert_eq!(
            vault
                .get_target_price(Uint128::new(20), 0, Precision::DecimalPlaces(3))
                .unwrap()
                .to_string(),
            "5"
        );
    }

    #[test]
    fn should_be_correct_when_selling_on_fin() {
        let vault = vault_with(Uint128::new(100), PositionType::Exit);
        assert_eq!(
            vault
                .get_target_price(Uint128::new(20), 0, Precision::DecimalPlaces(3))
                .unwrap()
                .to_string(),
            "0.2"
        );
    }

    #[test]
    fn should_truncate_price_to_three_decimal_places() {
        let vault = vault_with(Uint128::new(30), PositionType::Exit);
        assert_eq!(
            vault
                .get_target_price(Uint128::new(10), 0, Precision::DecimalPlaces(3))
                .unwrap()
                .to_string(),
            "0.333"
        );
    }

    #[test]
    fn for_fin_buy_with_decimal_delta_should_truncate() {
        let position_type = PositionType::Enter;
        let swap_amount = Uint128::new(1000000);
        let target_receive_amount = Uint128::new(747943156999999);
        let decimal_delta = 12;
        let precision = Precision::DecimalPlaces(2);
        let vault = vault_with(swap_amount, position_type);
        assert_eq!(
            Decimal256::from_ratio(swap_amount, target_receive_amount).to_string(),
            "0.000000001336999998"
        );
        assert_eq!(
            vault
                .get_target_price(target_receive_amount, decimal_delta, precision)
                .unwrap()
                .to_string(),
            "0.00000000133699"
        );
    }

    #[test]
    fn for_fin_sell_with_decimal_delta_should_truncate() {
        let position_type = PositionType::Exit;
        let swap_amount = Uint128::new(747943156999999);
        let target_receive_amount = Uint128::new(1000000);
        let decimal_delta = 12;
        let precision = Precision::DecimalPlaces(2);
        let vault = vault_with(swap_amount, position_type);
        assert_eq!(
            Decimal256::from_ratio(target_receive_amount, swap_amount).to_string(),
            "0.000000001336999998"
        );
        assert_eq!(
            vault
                .get_target_price(target_receive_amount, decimal_delta, precision)
                .unwrap()
                .to_string(),
            "0.00000000133699"
        );
    }

    fn vault_with(swap_amount: Uint128, position_type: PositionType) -> Vault {
        Vault {
            id: Uint128::new(1),
            created_at: Timestamp::from_seconds(0),
            owner: Addr::unchecked("owner"),
            label: None,
            destinations: vec![],
            status: VaultStatus::Active,
            balance: coin(
                1000,
                match position_type {
                    PositionType::Enter => "quote",
                    PositionType::Exit => "base",
                },
            ),
            pair: Pair {
                address: Addr::unchecked("pair"),
                base_denom: "base".to_string(),
                quote_denom: "quote".to_string(),
            },
            swap_amount,
            slippage_tolerance: None,
            minimum_receive_amount: None,
            time_interval: TimeInterval::Daily,
            started_at: None,
            swapped_amount: coin(
                0,
                match position_type {
                    PositionType::Enter => "quote",
                    PositionType::Exit => "base",
                },
            ),
            received_amount: coin(
                0,
                match position_type {
                    PositionType::Enter => "base",
                    PositionType::Exit => "quote",
                },
            ),
            trigger: None,
            dca_plus_config: None,
        }
    }
}
