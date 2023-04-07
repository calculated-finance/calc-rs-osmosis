use super::{
    dca_plus_config::DcaPlusConfig, destination::Destination, pair::Pair,
    position_type::PositionType, time_interval::TimeInterval, trigger::TriggerConfiguration,
};
use crate::helpers::time_helpers::get_total_execution_duration;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
use std::cmp::max;

#[cw_serde]
pub enum VaultStatus {
    Scheduled,
    Active,
    Inactive,
    Cancelled,
}

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
    pub slippage_tolerance: Option<Decimal>,
    pub minimum_receive_amount: Option<Uint128>,
    pub time_interval: TimeInterval,
    pub started_at: Option<Timestamp>,
    pub swapped_amount: Coin,
    pub received_amount: Coin,
    pub trigger: Option<TriggerConfiguration>,
    pub dca_plus_config: Option<DcaPlusConfig>,
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

    pub fn get_expected_execution_completed_date(&self, current_time: Timestamp) -> Timestamp {
        let remaining_balance =
            self.dca_plus_config
                .clone()
                .map_or(self.balance.amount, |dca_plus_config| {
                    max(
                        dca_plus_config.standard_dca_balance().amount,
                        self.balance.amount,
                    )
                });

        let execution_duration = get_total_execution_duration(
            current_time,
            remaining_balance
                .checked_div(self.swap_amount)
                .unwrap()
                .into(),
            &self.time_interval,
        );

        current_time.plus_seconds(
            execution_duration
                .num_seconds()
                .try_into()
                .expect("exected duration should be >= 0 seconds"),
        )
    }

    pub fn has_low_funds(&self) -> bool {
        self.balance.amount < self.swap_amount
    }

    pub fn is_dca_plus(&self) -> bool {
        self.dca_plus_config.is_some()
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

    pub fn is_finished_dca_plus_vault(&self) -> bool {
        self.is_inactive()
            && self.is_dca_plus()
            && self
                .dca_plus_config
                .clone()
                .map_or(false, |dca_plus_config| {
                    !dca_plus_config.has_sufficient_funds()
                })
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == VaultStatus::Cancelled
    }
}

#[cfg(test)]
mod get_expected_execution_completed_date_tests {
    use super::Vault;
    use crate::{
        constants::{ONE, TEN},
        tests::mocks::DENOM_UOSMO,
        types::{dca_plus_config::DcaPlusConfig, vault::VaultStatus},
    };
    use cosmwasm_std::{testing::mock_env, Coin, Decimal};

    #[test]
    fn expected_execution_end_date_is_now_when_vault_is_empty() {
        let env = mock_env();
        let vault = Vault {
            balance: Coin::new(0, DENOM_UOSMO),
            ..Vault::default()
        };

        assert_eq!(
            vault.get_expected_execution_completed_date(env.block.time),
            env.block.time
        );
    }

    #[test]
    fn expected_execution_end_date_is_in_future_when_vault_is_not_empty() {
        let env = mock_env();
        let vault = Vault::default();

        assert_eq!(
            vault.get_expected_execution_completed_date(env.block.time),
            env.block.time.plus_seconds(1000 / 100 * 24 * 60 * 60)
        );
    }

    #[test]
    fn expected_execution_end_date_is_at_end_of_standard_dca_execution() {
        let env = mock_env();
        let vault = Vault {
            status: VaultStatus::Inactive,
            balance: Coin::new(ONE.into(), DENOM_UOSMO),
            swap_amount: ONE,
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_UOSMO),
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        };

        assert_eq!(
            vault.get_expected_execution_completed_date(env.block.time),
            env.block.time.plus_seconds(9 * 24 * 60 * 60)
        );
    }

    #[test]
    fn expected_execution_end_date_is_at_end_of_dca_plus_execution() {
        let env = mock_env();
        let vault = Vault {
            balance: Coin::new((TEN - ONE).into(), DENOM_UOSMO),
            swap_amount: ONE,
            dca_plus_config: Some(DcaPlusConfig {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new((ONE + ONE + ONE).into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new((ONE + ONE + ONE).into(), DENOM_UOSMO),
                escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_UOSMO),
                ..DcaPlusConfig::default()
            }),
            ..Vault::default()
        };

        assert_eq!(
            vault.get_expected_execution_completed_date(env.block.time),
            env.block.time.plus_seconds(9 * 24 * 60 * 60)
        );
    }
}
