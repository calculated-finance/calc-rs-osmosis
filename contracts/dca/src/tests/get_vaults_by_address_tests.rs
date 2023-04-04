use crate::constants::{ONE, TEN};
use crate::contract::query;
use crate::msg::{QueryMsg, VaultsResponse};
use crate::tests::helpers::{instantiate_contract, setup_vault};
use crate::tests::mocks::{ADMIN, USER};
use base::vaults::vault::VaultStatus;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr, Uint128};

#[test]
fn with_no_vaults_should_return_all_vaults() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let vaults = from_binary::<VaultsResponse>(
        &query(
            deps.as_ref(),
            env,
            QueryMsg::GetVaultsByAddress {
                address: Addr::unchecked(USER),
                status: None,
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap()
    .vaults;

    assert_eq!(vaults.len(), 0);
}

#[test]
fn with_multiple_vaults_should_return_all_vaults() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let balance = TEN;
    let swap_amount = ONE;
    let status = VaultStatus::Active;
    let is_dca_plus = false;

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status.clone(),
        None,
        is_dca_plus,
    );

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status,
        None,
        is_dca_plus,
    );

    let vaults = from_binary::<VaultsResponse>(
        &query(
            deps.as_ref(),
            env,
            QueryMsg::GetVaultsByAddress {
                address: info.sender.clone(),
                status: None,
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap()
    .vaults;

    assert_eq!(vaults.len(), 2);
}

#[test]
fn with_limit_should_return_limited_vaults() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let balance = TEN;
    let swap_amount = ONE;
    let status = VaultStatus::Active;
    let is_dca_plus = false;

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status.clone(),
        None,
        is_dca_plus,
    );

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status,
        None,
        is_dca_plus,
    );

    let vaults = from_binary::<VaultsResponse>(
        &query(
            deps.as_ref(),
            env,
            QueryMsg::GetVaultsByAddress {
                address: info.sender.clone(),
                status: None,
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap()
    .vaults;

    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].id, Uint128::new(1));
}

#[test]
fn with_start_after_should_return_vaults_after_start_after() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let balance = TEN;
    let swap_amount = ONE;
    let status = VaultStatus::Active;
    let is_dca_plus = false;

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status.clone(),
        None,
        is_dca_plus,
    );

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status,
        None,
        is_dca_plus,
    );

    let vaults = from_binary::<VaultsResponse>(
        &query(
            deps.as_ref(),
            env,
            QueryMsg::GetVaultsByAddress {
                address: info.sender.clone(),
                status: None,
                start_after: Some(1),
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap()
    .vaults;

    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].id, Uint128::new(2));
}

#[test]
fn with_limit_and_start_after_should_return_limited_vaults_after_start_after() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let balance = TEN;
    let swap_amount = ONE;
    let status = VaultStatus::Active;
    let is_dca_plus = false;

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status.clone(),
        None,
        is_dca_plus,
    );

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status.clone(),
        None,
        is_dca_plus,
    );

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        status,
        None,
        is_dca_plus,
    );

    let vaults = from_binary::<VaultsResponse>(
        &query(
            deps.as_ref(),
            env,
            QueryMsg::GetVaultsByAddress {
                address: info.sender.clone(),
                status: None,
                start_after: Some(1),
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap()
    .vaults;

    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].id, Uint128::new(2));
}

#[test]
fn with_limit_too_large_should_fail() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let err = query(
        deps.as_ref(),
        env,
        QueryMsg::GetVaultsByAddress {
            address: info.sender.clone(),
            status: None,
            start_after: Some(1),
            limit: Some(10000),
        },
    )
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Generic error: limit cannot be greater than 1000."
    )
}

#[test]
fn with_status_filter_should_return_all_vaults_with_status() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info(ADMIN, &vec![]);

    instantiate_contract(deps.as_mut(), env.clone(), info.clone());

    let balance = TEN;
    let swap_amount = ONE;
    let is_dca_plus = false;

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        VaultStatus::Active,
        None,
        is_dca_plus,
    );

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        VaultStatus::Active,
        None,
        is_dca_plus,
    );

    setup_vault(
        deps.as_mut(),
        env.clone(),
        balance,
        swap_amount,
        VaultStatus::Scheduled,
        None,
        is_dca_plus,
    );

    let vaults = from_binary::<VaultsResponse>(
        &query(
            deps.as_ref(),
            env,
            QueryMsg::GetVaultsByAddress {
                address: info.sender.clone(),
                status: Some(VaultStatus::Active),
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap()
    .vaults;

    assert_eq!(vaults.len(), 2);
    vaults
        .iter()
        .for_each(|v| assert!(v.status == VaultStatus::Active));
}
