use super::mocks::DENOM_STAKE;
use crate::constants::{ONE, ONE_HUNDRED, TEN};
use crate::handlers::deposit::deposit;
use crate::handlers::get_events_by_resource_id::get_events_by_resource_id;
use crate::handlers::get_vault::get_vault;
use crate::msg::ExecuteMsg;
use crate::state::config::{get_config, update_config, Config};
use crate::tests::helpers::{instantiate_contract, setup_new_vault};
use crate::tests::mocks::{ADMIN, DENOM_UOSMO, USER};
use crate::types::dca_plus_config::DcaPlusConfig;
use crate::types::event::{EventBuilder, EventData};
use crate::types::vault::{Vault, VaultStatus};
use base::helpers::coin_helpers::{add, subtract};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, SubMsg, WasmMsg};

#[test]
fn updates_the_vault_balance() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            balance: Coin::new(0, DENOM_UOSMO),
            ..Vault::default()
        },
    );

    deposit(deps.as_mut(), env, info, vault.owner, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(
        vault.balance,
        subtract(&deposit_amount, &deposit_amount).unwrap()
    );
    assert_eq!(updated_vault.balance, deposit_amount);
}

#[test]
fn publishes_deposit_event() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap();

    let events = get_events_by_resource_id(deps.as_ref(), vault.id, None, None, None)
        .unwrap()
        .events;

    assert!(events.contains(
        &EventBuilder::new(
            vault.id,
            env.block,
            EventData::DcaVaultFundsDeposited {
                amount: deposit_amount,
            },
        )
        .build(1)
    ))
}

#[test]
fn updates_inactive_vault_to_active() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            ..Vault::default()
        },
    );

    deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(vault.status, VaultStatus::Inactive);
    assert_eq!(updated_vault.status, VaultStatus::Active);
}

#[test]
fn leaves_scheduled_vault_scheduled() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Scheduled,
            ..Vault::default()
        },
    );

    deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(vault.status, VaultStatus::Scheduled);
    assert_eq!(updated_vault.status, VaultStatus::Scheduled);
}

#[test]
fn leaves_active_vault_active() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(vault.status, VaultStatus::Active);
    assert_eq!(updated_vault.status, VaultStatus::Active);
}

#[test]
fn executes_trigger_for_reactivated_vault() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Inactive,
            trigger: None,
            ..Vault::default()
        },
    );

    let response = deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap();

    assert!(response
        .messages
        .contains(&SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::ExecuteTrigger {
                trigger_id: vault.id,
            })
            .unwrap(),
            funds: vec![],
        }))))
}

#[test]
fn does_not_execute_trigger_for_active_vault() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    let response = deposit(
        deps.as_mut(),
        env.clone(),
        info,
        Addr::unchecked(USER),
        vault.id,
    )
    .unwrap();

    assert!(response.messages.is_empty())
}

#[test]
fn does_not_execute_trigger_for_scheduled_vault() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Scheduled,
            ..Vault::default()
        },
    );

    let response = deposit(
        deps.as_mut(),
        env.clone(),
        info,
        Addr::unchecked(USER),
        vault.id,
    )
    .unwrap();

    assert!(response.messages.is_empty())
}

#[test]
fn for_cancelled_vault_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            status: VaultStatus::Cancelled,
            ..Vault::default()
        },
    );

    let err = deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap_err();

    assert_eq!(err.to_string(), "Error: vault is already cancelled");
}

#[test]
fn with_incorrect_denom_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_STAKE);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    let err = deposit(
        deps.as_mut(),
        env.clone(),
        mock_info(USER, &[Coin::new(ONE.into(), vault.received_amount.denom)]),
        vault.owner,
        vault.id,
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: received asset with denom stake, but needed uosmo"
    );
}

#[test]
fn with_multiple_assets_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_STAKE);
    let info = mock_info(
        ADMIN,
        &[deposit_amount.clone(), Coin::new(TEN.into(), DENOM_UOSMO)],
    );

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    let err = deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Error: received 2 denoms but required exactly 1"
    );
}

#[test]
fn when_contract_is_paused_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(TEN.into(), DENOM_STAKE);
    let info = mock_info(
        ADMIN,
        &[deposit_amount.clone(), Coin::new(TEN.into(), DENOM_UOSMO)],
    );

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let config = get_config(deps.as_ref().storage).unwrap();

    update_config(
        deps.as_mut().storage,
        Config {
            paused: true,
            ..config
        },
    )
    .unwrap();

    let vault = setup_new_vault(deps.as_mut(), env.clone(), Vault::default());

    let err = deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap_err();

    assert_eq!(err.to_string(), "Error: contract is paused");
}

#[test]
fn with_dca_plus_should_update_model_id() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(ONE_HUNDRED.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

    deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(vault.dca_plus_config.unwrap().model_id, 30);
    assert_eq!(updated_vault.dca_plus_config.unwrap().model_id, 80);
}

#[test]
fn with_dca_plus_should_update_total_deposit() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let deposit_amount = Coin::new(ONE_HUNDRED.into(), DENOM_UOSMO);
    let info = mock_info(ADMIN, &[deposit_amount.clone()]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vault = setup_new_vault(
        deps.as_mut(),
        env.clone(),
        Vault {
            dca_plus_config: Some(DcaPlusConfig::default()),
            ..Vault::default()
        },
    );

    deposit(deps.as_mut(), env.clone(), info, vault.owner, vault.id).unwrap();

    let updated_vault = get_vault(deps.as_ref(), vault.id).unwrap().vault;

    assert_eq!(vault.dca_plus_config.unwrap().total_deposit, vault.balance);
    assert_eq!(
        updated_vault.dca_plus_config.unwrap().total_deposit,
        add(vault.balance, deposit_amount).unwrap()
    );
}
