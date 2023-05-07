use super::{
    destination::Destination, performance_assessment_strategy::PerformanceAssessmentStrategy,
    position_type::PositionType, swap_adjustment_strategy::SwapAdjustmentStrategy,
    time_interval::TimeInterval, trigger::TriggerConfiguration,
};
use crate::helpers::time::get_total_execution_duration;
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
    pub started_at: Option<Timestamp>,
    pub owner: Addr,
    pub label: Option<String>,
    pub destinations: Vec<Destination>,
    pub status: VaultStatus,
    pub balance: Coin,
    pub target_denom: String,
    pub swap_amount: Uint128,
    pub slippage_tolerance: Option<Decimal>,
    pub minimum_receive_amount: Option<Uint128>,
    pub time_interval: TimeInterval,
    pub escrow_level: Decimal,
    pub deposited_amount: Coin,
    pub swapped_amount: Coin,
    pub received_amount: Coin,
    pub escrowed_amount: Coin,
    pub trigger: Option<TriggerConfiguration>,
    pub performance_assessment_strategy: Option<PerformanceAssessmentStrategy>,
    pub swap_adjustment_strategy: Option<SwapAdjustmentStrategy>,
}

impl Vault {
    pub fn denoms(&self) -> [String; 2] {
        [self.get_swap_denom(), self.target_denom.clone()]
    }

    pub fn get_swap_denom(&self) -> String {
        self.balance.denom.clone()
    }

    pub fn get_expected_execution_completed_date(&self, current_time: Timestamp) -> Timestamp {
        let remaining_balance = match self.performance_assessment_strategy.clone() {
            Some(PerformanceAssessmentStrategy::CompareToStandardDca {
                swapped_amount, ..
            }) => max(
                self.deposited_amount.amount - swapped_amount.amount,
                self.balance.amount,
            ),
            _ => self.balance.amount,
        };

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

    pub fn is_active(&self) -> bool {
        self.status == VaultStatus::Active
    }

    pub fn is_scheduled(&self) -> bool {
        self.status == VaultStatus::Scheduled
    }

    pub fn is_inactive(&self) -> bool {
        self.status == VaultStatus::Inactive
    }

    pub fn should_not_continue(&self) -> bool {
        self.is_inactive()
            && self.performance_assessment_strategy.clone().map_or(
                true,
                |performance_assessment_strategy| {
                    performance_assessment_strategy
                        .standard_dca_balance(self.deposited_amount.clone())
                        .amount
                        == Uint128::zero()
                },
            )
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == VaultStatus::Cancelled
    }
}

pub struct VaultBuilder {
    pub created_at: Timestamp,
    pub owner: Addr,
    pub label: Option<String>,
    pub destinations: Vec<Destination>,
    pub status: VaultStatus,
    pub balance: Coin,
    pub target_denom: String,
    pub swap_amount: Uint128,
    pub position_type: Option<PositionType>,
    pub slippage_tolerance: Option<Decimal>,
    pub minimum_receive_amount: Option<Uint128>,
    pub time_interval: TimeInterval,
    pub started_at: Option<Timestamp>,
    pub escrow_level: Decimal,
    pub deposited_amount: Coin,
    pub swapped_amount: Coin,
    pub received_amount: Coin,
    pub escrowed_amount: Coin,
    pub performance_assessment_strategy: Option<PerformanceAssessmentStrategy>,
    pub swap_adjustment_strategy: Option<SwapAdjustmentStrategy>,
}

impl VaultBuilder {
    pub fn new(
        created_at: Timestamp,
        owner: Addr,
        label: Option<String>,
        destinations: Vec<Destination>,
        status: VaultStatus,
        balance: Coin,
        target_denom: String,
        swap_amount: Uint128,
        position_type: Option<PositionType>,
        slippage_tolerance: Option<Decimal>,
        minimum_receive_amount: Option<Uint128>,
        time_interval: TimeInterval,
        started_at: Option<Timestamp>,
        escrow_level: Decimal,
        deposited_amount: Coin,
        swapped_amount: Coin,
        received_amount: Coin,
        escrowed_amount: Coin,
        performance_assessment_strategy: Option<PerformanceAssessmentStrategy>,
        swap_adjustment_strategy: Option<SwapAdjustmentStrategy>,
    ) -> VaultBuilder {
        VaultBuilder {
            created_at,
            owner,
            label,
            destinations,
            status,
            balance,
            target_denom,
            swap_amount,
            position_type,
            slippage_tolerance,
            minimum_receive_amount,
            time_interval,
            started_at,
            escrow_level,
            deposited_amount,
            swapped_amount,
            received_amount,
            escrowed_amount,
            performance_assessment_strategy,
            swap_adjustment_strategy,
        }
    }

    pub fn build(self, id: Uint128) -> Vault {
        Vault {
            id,
            created_at: self.created_at,
            started_at: self.started_at,
            owner: self.owner,
            label: self.label,
            destinations: self.destinations,
            status: self.status,
            balance: self.balance.clone(),
            target_denom: self.target_denom,
            swap_amount: self.swap_amount,
            slippage_tolerance: self.slippage_tolerance,
            minimum_receive_amount: self.minimum_receive_amount,
            time_interval: self.time_interval,
            escrow_level: self.escrow_level,
            deposited_amount: self.deposited_amount,
            swapped_amount: self.swapped_amount,
            received_amount: self.received_amount,
            escrowed_amount: self.escrowed_amount,
            performance_assessment_strategy: self.performance_assessment_strategy,
            swap_adjustment_strategy: self.swap_adjustment_strategy,
            trigger: None,
        }
    }
}

#[cfg(test)]
mod should_not_continue_tests {
    use crate::{
        constants::{ONE, TEN},
        tests::mocks::DENOM_UOSMO,
        types::{
            performance_assessment_strategy::PerformanceAssessmentStrategy,
            vault::{Vault, VaultStatus},
        },
    };
    use cosmwasm_std::Coin;

    #[test]
    fn when_regular_vault_is_active_is_false() {
        let vault = Vault::default();

        assert!(!vault.should_not_continue());
    }

    #[test]
    fn when_regular_vault_is_inactive_is_true() {
        let vault = Vault {
            status: VaultStatus::Inactive,
            ..Default::default()
        };

        assert!(vault.should_not_continue());
    }

    #[test]
    fn when_dca_vault_is_active_is_false() {
        let vault = Vault {
            performance_assessment_strategy: Some(Default::default()),
            ..Default::default()
        };

        assert!(!vault.should_not_continue());
    }

    #[test]
    fn when_dca_vault_is_inactive_and_standard_dca_is_active_is_false() {
        let vault = Vault {
            status: VaultStatus::Inactive,
            deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
            performance_assessment_strategy: Some(
                PerformanceAssessmentStrategy::CompareToStandardDca {
                    swapped_amount: Coin::new((TEN - ONE).into(), DENOM_UOSMO),
                    received_amount: Coin::new((TEN - ONE).into(), DENOM_UOSMO),
                },
            ),
            ..Default::default()
        };

        assert!(!vault.should_not_continue());
    }

    #[test]
    fn when_dca_vault_is_inactive_and_standard_dca_is_inactive_is_true() {
        let vault = Vault {
            status: VaultStatus::Inactive,
            deposited_amount: Coin::new(TEN.into(), DENOM_UOSMO),
            performance_assessment_strategy: Some(
                PerformanceAssessmentStrategy::CompareToStandardDca {
                    swapped_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                    received_amount: Coin::new(TEN.into(), DENOM_UOSMO),
                },
            ),
            ..Default::default()
        };

        assert!(vault.should_not_continue());
    }
}

#[cfg(test)]
mod get_expected_execution_completed_date_tests {
    use super::Vault;
    use crate::{
        constants::{ONE, TEN},
        tests::mocks::DENOM_UOSMO,
        types::{
            performance_assessment_strategy::PerformanceAssessmentStrategy, vault::VaultStatus,
        },
    };
    use cosmwasm_std::{testing::mock_env, Coin};

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
            performance_assessment_strategy: Some(
                PerformanceAssessmentStrategy::CompareToStandardDca {
                    swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                    received_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                },
            ),
            ..Vault::default()
        };

        assert_eq!(
            vault.get_expected_execution_completed_date(env.block.time),
            env.block.time.plus_seconds(9 * 24 * 60 * 60)
        );
    }

    #[test]
    fn expected_execution_end_date_is_at_end_of_performance_assessment() {
        let env = mock_env();
        let vault = Vault {
            balance: Coin::new((TEN - ONE).into(), DENOM_UOSMO),
            swap_amount: ONE,
            performance_assessment_strategy: Some(
                PerformanceAssessmentStrategy::CompareToStandardDca {
                    swapped_amount: Coin::new((ONE + ONE + ONE).into(), DENOM_UOSMO),
                    received_amount: Coin::new((ONE + ONE + ONE).into(), DENOM_UOSMO),
                },
            ),
            ..Vault::default()
        };

        assert_eq!(
            vault.get_expected_execution_completed_date(env.block.time),
            env.block.time.plus_seconds(9 * 24 * 60 * 60)
        );
    }
}
