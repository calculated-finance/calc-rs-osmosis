#[cfg(test)]
pub mod mocks;

#[cfg(test)]
pub mod helpers;

#[cfg(test)]
pub mod contract_tests;

#[cfg(test)]
pub mod integration_create_vault_with_time_trigger_tests;

#[cfg(test)]
pub mod integration_create_vault_with_fin_limit_order_trigger_tests;

#[cfg(test)]
pub mod integration_execute_time_trigger_by_id_tests;

#[cfg(test)]
pub mod integration_execute_fin_limit_order_trigger_by_order_idx_tests;

#[cfg(test)]
pub mod integration_cancel_vault_by_address_and_id_tests;
