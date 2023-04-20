use crate::state::vaults::get_vaults_by_address as fetch_vaults_by_address;
use crate::types::vault::VaultStatus;
use crate::{helpers::validation::assert_page_limit_is_valid, msg::VaultsResponse};
use cosmwasm_std::{Addr, Deps, StdResult};

pub fn get_vaults_by_address_handler(
    deps: Deps,
    address: Addr,
    status: Option<VaultStatus>,
    start_after: Option<u128>,
    limit: Option<u16>,
) -> StdResult<VaultsResponse> {
    deps.api.addr_validate(address.as_ref())?;
    assert_page_limit_is_valid(deps.storage, limit)?;

    let vaults = fetch_vaults_by_address(deps.storage, address, status, start_after, limit)?;

    Ok(VaultsResponse { vaults })
}

#[cfg(test)]
mod get_vaults_by_address_tests {
    use crate::contract::query;
    use crate::msg::{QueryMsg, VaultsResponse};
    use crate::tests::helpers::{instantiate_contract, setup_vault};
    use crate::tests::mocks::ADMIN;
    use crate::types::vault::{Vault, VaultStatus};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Uint128};

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
                    address: Vault::default().owner,
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

        setup_vault(deps.as_mut(), env.clone(), Vault::default());
        setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let vaults = from_binary::<VaultsResponse>(
            &query(
                deps.as_ref(),
                env,
                QueryMsg::GetVaultsByAddress {
                    address: Vault::default().owner,
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

        setup_vault(deps.as_mut(), env.clone(), Vault::default());
        setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let vaults = from_binary::<VaultsResponse>(
            &query(
                deps.as_ref(),
                env,
                QueryMsg::GetVaultsByAddress {
                    address: Vault::default().owner,
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
        assert_eq!(vaults[0].id, Uint128::new(0));
    }

    #[test]
    fn with_start_after_should_return_vaults_after_start_after() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        setup_vault(deps.as_mut(), env.clone(), Vault::default());
        setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let vaults = from_binary::<VaultsResponse>(
            &query(
                deps.as_ref(),
                env,
                QueryMsg::GetVaultsByAddress {
                    address: Vault::default().owner,
                    status: None,
                    start_after: Some(0),
                    limit: None,
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
    fn with_limit_and_start_after_should_return_limited_vaults_after_start_after() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADMIN, &vec![]);

        instantiate_contract(deps.as_mut(), env.clone(), info.clone());

        setup_vault(deps.as_mut(), env.clone(), Vault::default());
        setup_vault(deps.as_mut(), env.clone(), Vault::default());
        setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let vaults = from_binary::<VaultsResponse>(
            &query(
                deps.as_ref(),
                env,
                QueryMsg::GetVaultsByAddress {
                    address: Vault::default().owner,
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
                address: Vault::default().owner,
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

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Active,
                ..Vault::default()
            },
        );

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Active,
                ..Vault::default()
            },
        );

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                status: VaultStatus::Scheduled,
                ..Vault::default()
            },
        );

        let vaults = from_binary::<VaultsResponse>(
            &query(
                deps.as_ref(),
                env,
                QueryMsg::GetVaultsByAddress {
                    address: Vault::default().owner,
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
}
