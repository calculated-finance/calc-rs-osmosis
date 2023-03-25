use super::dca_plus_config::DcaPlusConfig;
use base::{
    helpers::time_helpers::get_total_execution_duration,
    pool::Pool,
    triggers::trigger::{TimeInterval, TriggerConfiguration},
    vaults::vault::{Destination, VaultStatus},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
use osmosis_helpers::position_type::PositionType;
use std::{cmp::max};

#[cw_serde]
pub struct Vault {
    pub id: Uint128,
    pub created_at: Timestamp,
    pub owner: Addr,
    pub label: Option<String>,
    pub destinations: Vec<Destination>,
    pub status: VaultStatus,
    pub balance: Coin,
    pub pool: Pool,
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
        match self.balance.denom == self.pool.quote_denom {
            true => PositionType::Enter,
            false => PositionType::Exit,
        }
    }

    pub fn get_swap_denom(&self) -> String {
        self.balance.denom.clone()
    }

    pub fn get_receive_denom(&self) -> String {
        if self.balance.denom == self.pool.quote_denom {
            return self.pool.base_denom.clone();
        }
        self.pool.quote_denom.clone()
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

    pub fn has_sufficient_funds(&self) -> bool {
        let swap_amount = match self.has_low_funds() {
            true => self.balance.amount,
            false => self.swap_amount,
        };

        swap_amount > Uint128::new(50000)
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

// #[cfg(test)]
// mod has_sufficient_funds_tests {
//     use crate::{state::vaults::save_vault, types::vault_builder::VaultBuilder};

//     use super::*;
//     use cosmwasm_std::{coin, testing::mock_dependencies};

//     #[test]
//     fn should_return_false_when_vault_has_insufficient_swap_amount() {
//         let mut deps = mock_dependencies();
//         let vault_builder = vault_with(100000, Uint128::new(50000));
//         let vault = save_vault(deps.as_mut().storage, vault_builder).unwrap();
//         assert!(!vault.has_sufficient_funds());
//     }

//     #[test]
//     fn should_return_false_when_vault_has_insufficient_balance() {
//         let mut deps = mock_dependencies();
//         let vault_builder = vault_with(50000, Uint128::new(50001));
//         let vault = save_vault(deps.as_mut().storage, vault_builder).unwrap();
//         assert!(!vault.has_sufficient_funds());
//     }

//     #[test]
//     fn should_return_true_when_vault_has_sufficient_swap_amount() {
//         let mut deps = mock_dependencies();
//         let vault_builder = vault_with(100000, Uint128::new(50001));
//         let vault = save_vault(deps.as_mut().storage, vault_builder).unwrap();
//         assert!(vault.has_sufficient_funds());
//     }

//     #[test]
//     fn should_return_true_when_vault_has_sufficient_balance() {
//         let mut deps = mock_dependencies();
//         let vault_builder = vault_with(50001, Uint128::new(50002));
//         let vault = save_vault(deps.as_mut().storage, vault_builder).unwrap();
//         assert!(vault.has_sufficient_funds());
//     }

//     fn vault_with(balance: u128, swap_amount: Uint128) -> VaultBuilder {
//         VaultBuilder::new(
//             Timestamp::from_seconds(0),
//             Addr::unchecked("owner"),
//             None,
//             vec![],
//             VaultStatus::Active,
//             coin(balance, "quote"),
//             Pool {
//                 pool_id: 0,
//                 base_denom: "base".to_string(),
//                 quote_denom: "quote".to_string(),
//             },
//             swap_amount,
//             None,
//             None,
//             None,
//             TimeInterval::Daily,
//             None,
//             Coin {
//                 denom: "quote".to_string(),
//                 amount: Uint128::new(0),
//             },
//             Coin {
//                 denom: "base".to_string(),
//                 amount: Uint128::new(0),
//             },
//             None,
//         )
//     }
// }

// #[cfg(test)]
// mod get_target_price_tests {
//     use super::*;
//     use cosmwasm_std::coin;

//     #[test]
//     fn should_be_correct_when_buying_on_fin() {
//         let vault = vault_with(Uint128::new(100), PositionType::Enter);
//         assert_eq!(
//             vault
//                 .get_target_price(Uint128::new(20), 0, Precision::DecimalPlaces(3))
//                 .unwrap()
//                 .to_string(),
//             "5"
//         );
//     }

//     #[test]
//     fn should_be_correct_when_selling_on_fin() {
//         let vault = vault_with(Uint128::new(100), PositionType::Exit);
//         assert_eq!(
//             vault
//                 .get_target_price(Uint128::new(20), 0, Precision::DecimalPlaces(3))
//                 .unwrap()
//                 .to_string(),
//             "0.2"
//         );
//     }

//     #[test]
//     fn should_truncate_price_to_three_decimal_places() {
//         let vault = vault_with(Uint128::new(30), PositionType::Exit);
//         assert_eq!(
//             vault
//                 .get_target_price(Uint128::new(10), 0, Precision::DecimalPlaces(3))
//                 .unwrap()
//                 .to_string(),
//             "0.333"
//         );
//     }

//     #[test]
//     fn for_fin_buy_with_decimal_delta_should_truncate() {
//         let position_type = PositionType::Enter;
//         let swap_amount = Uint128::new(1000000);
//         let target_receive_amount = Uint128::new(747943156999999);
//         let decimal_delta = 12;
//         let precision = Precision::DecimalPlaces(2);
//         let vault = vault_with(swap_amount, position_type);
//         assert_eq!(
//             Decimal256::from_ratio(swap_amount, target_receive_amount).to_string(),
//             "0.000000001336999998"
//         );
//         assert_eq!(
//             vault
//                 .get_target_price(target_receive_amount, decimal_delta, precision)
//                 .unwrap()
//                 .to_string(),
//             "0.00000000133699"
//         );
//     }

//     #[test]
//     fn for_fin_sell_with_decimal_delta_should_truncate() {
//         let position_type = PositionType::Exit;
//         let swap_amount = Uint128::new(747943156999999);
//         let target_receive_amount = Uint128::new(1000000);
//         let decimal_delta = 12;
//         let precision = Precision::DecimalPlaces(2);
//         let vault = vault_with(swap_amount, position_type);
//         assert_eq!(
//             Decimal256::from_ratio(target_receive_amount, swap_amount).to_string(),
//             "0.000000001336999998"
//         );
//         assert_eq!(
//             vault
//                 .get_target_price(target_receive_amount, decimal_delta, precision)
//                 .unwrap()
//                 .to_string(),
//             "0.00000000133699"
//         );
//     }

//     fn vault_with(swap_amount: Uint128, position_type: PositionType) -> Vault {
//         Vault {
//             id: Uint128::new(1),
//             created_at: Timestamp::from_seconds(0),
//             owner: Addr::unchecked("owner"),
//             label: None,
//             destinations: vec![],
//             status: VaultStatus::Active,
//             balance: coin(
//                 1000,
//                 match position_type {
//                     PositionType::Enter => "quote",
//                     PositionType::Exit => "base",
//                 },
//             ),
//             pair: Pool {
//                 pool_id: 0,
//                 base_denom: "base".to_string(),
//                 quote_denom: "quote".to_string(),
//             },
//             swap_amount,
//             slippage_tolerance: None,
//             minimum_receive_amount: None,
//             time_interval: TimeInterval::Daily,
//             started_at: None,
//             swapped_amount: coin(
//                 0,
//                 match position_type {
//                     PositionType::Enter => "quote",
//                     PositionType::Exit => "base",
//                 },
//             ),
//             received_amount: coin(
//                 0,
//                 match position_type {
//                     PositionType::Enter => "base",
//                     PositionType::Exit => "quote",
//                 },
//             ),
//             trigger: None,
//             dca_plus_config: None,
//         }
//     }
// }

// #[cfg(test)]
// mod get_expected_execution_completed_date_tests {
//     use crate::{
//         constants::{ONE, TEN},
//         tests::mocks::DENOM_UKUJI,
//         types::dca_plus_config::DcaPlusConfig,
//     };

//     use super::Vault;
//     use base::{pool::Pool, triggers::trigger::TimeInterval, vaults::vault::VaultStatus};
//     use cosmwasm_std::{coin, testing::mock_env, Addr, Coin, Decimal, Timestamp, Uint128};

//     #[test]
//     fn expected_execution_end_date_is_now_when_vault_is_empty() {
//         let env = mock_env();
//         let created_at = env.block.time.minus_seconds(60 * 60 * 24);
//         let vault = vault_with(
//             created_at,
//             Uint128::zero(),
//             Uint128::new(100),
//             TimeInterval::Daily,
//         );
//         assert_eq!(
//             vault.get_expected_execution_completed_date(env.block.time),
//             env.block.time
//         );
//     }

//     #[test]
//     fn expected_execution_end_date_is_in_future_when_vault_is_not_empty() {
//         let env = mock_env();
//         let vault = vault_with(
//             env.block.time,
//             Uint128::new(1000),
//             Uint128::new(100),
//             TimeInterval::Daily,
//         );
//         assert_eq!(
//             vault.get_expected_execution_completed_date(env.block.time),
//             env.block.time.plus_seconds(1000 / 100 * 24 * 60 * 60)
//         );
//     }

//     #[test]
//     fn expected_execution_end_date_is_at_end_of_standard_dca_execution() {
//         let env = mock_env();
//         let mut vault = vault_with(env.block.time, Uint128::zero(), ONE, TimeInterval::Daily);

//         vault.status = VaultStatus::Inactive;

//         vault.dca_plus_config = Some(DcaPlusConfig {
//             escrow_level: Decimal::percent(5),
//             model_id: 30,
//             total_deposit: Coin::new(TEN.into(), DENOM_UKUJI),
//             standard_dca_swapped_amount: Coin::new(ONE.into(), DENOM_UKUJI),
//             standard_dca_received_amount: Coin::new(ONE.into(), DENOM_UKUJI),
//             escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_UKUJI),
//         });

//         assert_eq!(
//             vault.get_expected_execution_completed_date(env.block.time),
//             env.block.time.plus_seconds(9 * 24 * 60 * 60)
//         );
//     }

//     #[test]
//     fn expected_execution_end_date_is_at_end_of_dca_plus_execution() {
//         let env = mock_env();
//         let mut vault = vault_with(env.block.time, TEN - ONE, ONE, TimeInterval::Daily);

//         vault.dca_plus_config = Some(DcaPlusConfig {
//             escrow_level: Decimal::percent(5),
//             model_id: 30,
//             total_deposit: Coin::new(TEN.into(), DENOM_UKUJI),
//             standard_dca_swapped_amount: Coin::new((ONE + ONE + ONE).into(), DENOM_UKUJI),
//             standard_dca_received_amount: Coin::new((ONE + ONE + ONE).into(), DENOM_UKUJI),
//             escrowed_balance: Coin::new((ONE * Decimal::percent(5)).into(), DENOM_UKUJI),
//         });

//         assert_eq!(
//             vault.get_expected_execution_completed_date(env.block.time),
//             env.block.time.plus_seconds(9 * 24 * 60 * 60)
//         );
//     }

//     fn vault_with(
//         created_at: Timestamp,
//         balance: Uint128,
//         swap_amount: Uint128,
//         time_interval: TimeInterval,
//     ) -> Vault {
//         Vault {
//             id: Uint128::new(1),
//             created_at,
//             owner: Addr::unchecked("owner"),
//             label: None,
//             destinations: vec![],
//             status: VaultStatus::Active,
//             balance: Coin::new(balance.into(), "quote"),
//             pair: Pool {
//                 pool_id: 0,
//                 base_denom: "base".to_string(),
//                 quote_denom: "quote".to_string(),
//             },
//             swap_amount,
//             slippage_tolerance: None,
//             minimum_receive_amount: None,
//             time_interval,
//             started_at: None,
//             swapped_amount: coin(0, "quote"),
//             received_amount: coin(0, "base"),
//             trigger: None,
//             dca_plus_config: None,
//         }
//     }
// }
