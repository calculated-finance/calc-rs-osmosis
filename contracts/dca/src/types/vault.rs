use base::{
    pair::Pair,
    triggers::trigger::{TimeInterval, TriggerConfiguration},
    vaults::vault::{Destination, PositionType, VaultStatus},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal256, Timestamp, Uint128};

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

    pub fn get_swap_amount(&self) -> Coin {
        Coin {
            denom: self.get_swap_denom(),
            amount: match self.low_funds() {
                true => self.balance.amount,
                false => self.swap_amount,
            },
        }
    }

    pub fn price_threshold_exceeded(&self, price: Decimal256) -> bool {
        if let Some(minimum_receive_amount) = self.minimum_receive_amount {
            let receive_amount_at_price = match self.get_position_type() {
                PositionType::Enter => Decimal256::from_ratio(self.swap_amount, Uint128::one())
                    .checked_div(price)
                    .expect("current fin price should be > 0.0"),
                PositionType::Exit => Decimal256::from_ratio(self.swap_amount, Uint128::one())
                    .checked_mul(price)
                    .expect("expected receive amount should be valid"),
            };

            let minimum_receive_amount =
                Decimal256::from_ratio(minimum_receive_amount, Uint128::one());

            return receive_amount_at_price < minimum_receive_amount;
        }

        false
    }

    pub fn low_funds(&self) -> bool {
        self.balance.amount < self.swap_amount
    }

    pub fn is_empty(&self) -> bool {
        self.balance.amount.is_zero()
    }

    pub fn is_active(&self) -> bool {
        self.status == VaultStatus::Active
    }

    pub fn is_scheduled(&self) -> bool {
        self.status == VaultStatus::Scheduled
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
        }
    }
}
