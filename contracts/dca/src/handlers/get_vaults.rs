use crate::state::vaults::get_vaults;
use crate::{helpers::validation::assert_page_limit_is_valid, msg::VaultsResponse};
use cosmwasm_std::{Deps, StdResult};

pub fn get_vaults_handler(
    deps: Deps,
    start_after: Option<u128>,
    limit: Option<u16>,
) -> StdResult<VaultsResponse> {
    assert_page_limit_is_valid(deps.storage, limit)?;

    let vaults = get_vaults(deps.storage, start_after, limit)?;

    Ok(VaultsResponse { vaults })
}

#[cfg(test)]
mod get_vaults_tests {
    use super::*;
    use crate::tests::helpers::{instantiate_contract, setup_vault};
    use crate::tests::mocks::ADMIN;
    use crate::types::vault::Vault;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Uint128;

    #[test]
    fn with_limit_too_large_should_fail() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let err = get_vaults_handler(deps.as_ref(), None, Some(1001)).unwrap_err();

        assert_eq!(
            err.to_string(),
            "Generic error: limit cannot be greater than 1000."
        );
    }

    #[test]
    fn with_no_vaults_should_return_all_vaults() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let vaults = get_vaults_handler(deps.as_ref(), None, None)
            .unwrap()
            .vaults;

        assert_eq!(vaults.len(), 0);
    }

    #[test]
    fn with_multiple_vaults_should_return_all_vaults() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(1),
                ..Vault::default()
            },
        );

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(2),
                ..Vault::default()
            },
        );

        let vaults = get_vaults_handler(deps.as_ref(), None, None)
            .unwrap()
            .vaults;

        assert_eq!(vaults.len(), 2);
    }

    #[test]
    fn with_one_vault_should_return_proper_vault_data() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        let vault = setup_vault(deps.as_mut(), env.clone(), Vault::default());

        let vaults = get_vaults_handler(deps.as_ref(), None, None)
            .unwrap()
            .vaults;

        assert_eq!(vaults.first().unwrap(), &vault);
    }

    #[test]
    fn with_limit_should_return_limited_vaults() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(1),
                ..Vault::default()
            },
        );

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(2),
                ..Vault::default()
            },
        );

        let vaults = get_vaults_handler(deps.as_ref(), None, Some(1))
            .unwrap()
            .vaults;

        assert_eq!(vaults.len(), 1);
        assert_eq!(vaults[0].id, Uint128::new(1));
    }

    #[test]
    fn with_start_after_should_return_vaults_after_start_after() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(1),
                ..Vault::default()
            },
        );

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(2),
                ..Vault::default()
            },
        );

        let vaults = get_vaults_handler(deps.as_ref(), Some(1), None)
            .unwrap()
            .vaults;

        assert_eq!(vaults.len(), 1);
        assert_eq!(vaults[0].id, Uint128::new(2));
    }

    #[test]
    fn with_limit_and_start_after_should_return_limited_vaults_after_start_after() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate_contract(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]));

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(1),
                ..Vault::default()
            },
        );

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(2),
                ..Vault::default()
            },
        );

        setup_vault(
            deps.as_mut(),
            env.clone(),
            Vault {
                id: Uint128::new(3),
                ..Vault::default()
            },
        );

        let vaults = get_vaults_handler(deps.as_ref(), Some(1), Some(1))
            .unwrap()
            .vaults;

        assert_eq!(vaults.len(), 1);
        assert_eq!(vaults[0].id, Uint128::new(2));
    }
}
