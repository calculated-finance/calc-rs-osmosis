#[cfg(test)]
pub mod mocks;

#[cfg(test)]
pub mod helpers;

#[cfg(test)]
pub mod contract_tests;

#[cfg(test)]
pub mod create_vault_tests;

#[cfg(test)]
pub mod cancel_vault_tests;

#[cfg(test)]
pub mod execute_trigger_tests;

#[cfg(test)]
pub mod get_time_trigger_ids_tests;

#[cfg(test)]
pub mod get_trigger_id_by_fin_limit_order_idx_tests;

#[cfg(test)]
pub mod get_vaults_by_address_tests;

#[cfg(test)]
pub mod deposit_tests;

#[cfg(test)]
pub mod update_config_tests;

#[cfg(test)]
pub mod after_fin_limit_order_retracted_tests;

#[cfg(test)]
pub mod after_fin_swap_tests;

#[cfg(test)]
pub mod get_events_by_resource_id_tests;

#[cfg(test)]
pub mod after_fin_limit_order_withdrawn_for_cancel_vault_tests;

#[cfg(test)]
pub mod update_vault_tests;

#[cfg(test)]
pub mod after_fin_limit_order_withdrawn_for_execute_vault_tests;

#[cfg(test)]
pub mod create_custom_swap_fee_tests;

#[cfg(test)]
pub mod remove_custom_fee_tests;

#[cfg(test)]
pub mod get_vaults_tests;

#[cfg(test)]
pub mod get_config_tests;

#[cfg(test)]
pub mod disburse_escrow_tests;

#[cfg(test)]
pub mod update_swap_adjustments_tests;
