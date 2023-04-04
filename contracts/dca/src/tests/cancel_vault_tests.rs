use super::helpers::{instantiate_contract, setup_vault};
use crate::handlers::cancel_vault::cancel_vault;
use crate::handlers::get_events_by_resource_id::get_events_by_resource_id;
use crate::handlers::get_vault::get_vault;
use crate::state::disburse_escrow_tasks::get_disburse_escrow_tasks;
use crate::tests::mocks::ADMIN;
use base::events::event::{EventBuilder, EventData};
use base::vaults::vault::VaultStatus;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{BankMsg, CosmosMsg, SubMsg, Uint128};

#[test]
fn should_return_balance_to_owner() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let balance = Uint128::new(1000000);

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        Uint128::new(100000),
        VaultStatus::Active,
        None,
        false,
    );

    let response = cancel_vault(deps.as_mut(), env, info, vault.id).unwrap();

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: vault.owner.to_string(),
            amount: vec![vault.balance.clone()],
        }))));
}

#[test]
fn should_publish_vault_cancelled_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Uint128::new(1000000),
        Uint128::new(100000),
        VaultStatus::Active,
        None,
        false,
    );

    cancel_vault(deps.as_mut(), env.clone(), info, vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None)
        .unwrap()
        .events;

    assert!(events.contains(
        &EventBuilder::new(vault.id, env.block, EventData::DcaVaultCancelled {}).build(1)
    ));
}

#[test]
fn when_vault_has_time_trigger_should_cancel_vault() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Uint128::new(1000000),
        Uint128::new(100000),
        VaultStatus::Active,
        None,
        false,
    );

    cancel_vault(deps.as_mut(), env.clone(), info, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(vault.status, VaultStatus::Active);
    assert_eq!(updated_vault.status, VaultStatus::Cancelled);
}

#[test]
fn should_empty_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let balance = Uint128::new(1000000);

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        Uint128::new(100000),
        VaultStatus::Active,
        None,
        false,
    );

    cancel_vault(deps.as_mut(), env.clone(), info, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(vault.balance.amount, balance);
    assert_eq!(updated_vault.balance.amount, Uint128::zero());
}

#[test]
fn on_already_cancelled_vault_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Uint128::new(1000000),
        Uint128::new(100000),
        VaultStatus::Active,
        None,
        false,
    );

    cancel_vault(deps.as_mut(), env.clone(), info.clone(), vault.id).unwrap();
    let err = cancel_vault(deps.as_mut(), env.clone(), info, vault.id).unwrap_err();

    assert_eq!(err.to_string(), "Error: vault is already cancelled");
}

#[test]
fn for_vault_with_different_owner_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Uint128::new(1000000),
        Uint128::new(100000),
        VaultStatus::Active,
        None,
        false,
    );

    let err = cancel_vault(
        deps.as_mut(),
        env.clone(),
        mock_info("not-the-owner", &[]),
        vault.id,
    )
    .unwrap_err();

    assert_eq!(err.to_string(), "Unauthorized");
}

#[test]
fn should_delete_the_trigger() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Uint128::new(1000000),
        Uint128::new(100000),
        VaultStatus::Active,
        None,
        false,
    );

    cancel_vault(deps.as_mut(), env.clone(), info, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_ne!(vault.trigger, None);
    assert_eq!(updated_vault.trigger, None);
}

#[test]
fn with_dca_plus_should_save_disburse_escrow_task() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &[]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_vault(
        deps.as_mut(),
        env.clone(),
        Uint128::new(1000000),
        Uint128::new(100000),
        VaultStatus::Active,
        None,
        true,
    );

    cancel_vault(deps.as_mut(), env.clone(), info, vault.id).unwrap();

    let disburse_escrow_tasks_before = get_disburse_escrow_tasks(
        deps.as_ref().storage,
        vault
            .get_expected_execution_completed_date(env.block.time)
            .minus_seconds(10),
        Some(100),
    )
    .unwrap();

    assert!(disburse_escrow_tasks_before.is_empty());

    let disburse_escrow_tasks_after = get_disburse_escrow_tasks(
        deps.as_ref().storage,
        vault
            .get_expected_execution_completed_date(env.block.time)
            .plus_seconds(10),
        Some(100),
    )
    .unwrap();

    assert!(disburse_escrow_tasks_after.contains(&vault.id));
}
