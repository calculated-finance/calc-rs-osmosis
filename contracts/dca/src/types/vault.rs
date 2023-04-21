use super::{
    destination::Destination, position_type::PositionType,
    swap_adjustment_strategy::SwapAdjustmentStrategy, time_interval::TimeInterval,
    trigger::TriggerConfiguration,
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
    pub started_at: Option<Timestamp>,
    pub swapped_amount: Coin,
    pub received_amount: Coin,
    pub trigger: Option<TriggerConfiguration>,
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
        let remaining_balance = match self.swap_adjustment_strategy.clone() {
            Some(SwapAdjustmentStrategy::DcaPlus {
                total_deposit,
                standard_dca_swapped_amount,
                ..
            }) => max(
                total_deposit.amount - standard_dca_swapped_amount.amount,
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

    pub fn has_low_funds(&self) -> bool {
        self.balance.amount < self.swap_amount
    }

    pub fn is_dca_plus(&self) -> bool {
        self.swap_adjustment_strategy.is_some()
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
                .swap_adjustment_strategy
                .clone()
                .map_or(false, |swap_adjustment_strategy| {
                    !swap_adjustment_strategy.can_continue()
                })
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
    pub swapped_amount: Coin,
    pub received_amount: Coin,
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
        swapped_amount: Coin,
        received_amount: Coin,
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
            swapped_amount,
            received_amount,
            swap_adjustment_strategy,
        }
    }

    pub fn build(self, id: Uint128) -> Vault {
        Vault {
            id,
            created_at: self.created_at,
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
            started_at: self.started_at,
            swapped_amount: self.swapped_amount,
            received_amount: self.received_amount,
            trigger: None,
            swap_adjustment_strategy: self.swap_adjustment_strategy,
        }
    }
}

#[cfg(test)]
mod get_expected_execution_completed_date_tests {
    use super::Vault;
    use crate::{
        constants::{ONE, TEN},
        tests::mocks::DENOM_UOSMO,
        types::{swap_adjustment_strategy::SwapAdjustmentStrategy, vault::VaultStatus},
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
            swap_adjustment_strategy: Some(SwapAdjustmentStrategy::DcaPlus {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new(ONE.into(), DENOM_UOSMO),
                escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_UOSMO),
                model_id: 30,
                escrow_level: Decimal::percent(5),
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
            swap_adjustment_strategy: Some(SwapAdjustmentStrategy::DcaPlus {
                total_deposit: Coin::new(TEN.into(), DENOM_UOSMO),
                standard_dca_swapped_amount: Coin::new((ONE + ONE + ONE).into(), DENOM_UOSMO),
                standard_dca_received_amount: Coin::new((ONE + ONE + ONE).into(), DENOM_UOSMO),
                escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_UOSMO),
                model_id: 30,
                escrow_level: Decimal::percent(5),
            }),
            ..Vault::default()
        };

        assert_eq!(
            vault.get_expected_execution_completed_date(env.block.time),
            env.block.time.plus_seconds(9 * 24 * 60 * 60)
        );
    }
}
