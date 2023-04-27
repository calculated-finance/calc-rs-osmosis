use crate::{
    error::ContractError,
    helpers::validation::{
        assert_label_is_no_longer_than_100_characters, assert_vault_is_not_cancelled,
        asset_sender_is_vault_owner,
    },
    state::vaults::{get_vault, update_vault},
};
use cosmwasm_std::{DepsMut, MessageInfo, Response, Uint128};

pub fn update_vault_handler(
    deps: DepsMut,
    info: MessageInfo,
    vault_id: Uint128,
    label: Option<String>,
) -> Result<Response, ContractError> {
    assert_label_is_no_longer_than_100_characters(&label)?;

    let mut vault = get_vault(deps.storage, vault_id)?;

    assert_vault_is_not_cancelled(&vault)?;

    vault.label = label.clone();
    update_vault(deps.storage, &vault)?;

    asset_sender_is_vault_owner(vault.owner, info.sender)?;

    Ok(Response::default()
        .add_attribute("update_vault", "true")
        .add_attribute("vault_id", vault.id)
        .add_attribute("label", label.unwrap_or_default()))
}

#[cfg(test)]
mod update_vault_tests {
    use super::update_vault_handler;
    use crate::{
        state::vaults::get_vault,
        tests::{helpers::setup_vault, mocks::USER},
        types::vault::{Vault, VaultStatus},
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr,
    };

    #[test]
    fn with_label_longer_than_100_characters() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let label = Some("12345678910".repeat(10).to_string());

        let err =
            update_vault_handler(deps.as_mut(), mock_info(USER, &[]), vault.id, label.clone())
                .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: Vault label cannot be longer than 100 characters"
        );
    }

    #[test]
    fn for_vault_with_different_owner_should_fail() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(
            deps.as_mut(),
            mock_env(),
            Vault {
                owner: Addr::unchecked("random"),
                ..Vault::default()
            },
        );

        let label = Some("My new vault".to_string());

        let err =
            update_vault_handler(deps.as_mut(), mock_info(USER, &[]), vault.id, label.clone())
                .unwrap_err();

        assert_eq!(err.to_string(), "Unauthorized");
    }

    #[test]
    fn for_cancelled_vault_should_fail() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(
            deps.as_mut(),
            mock_env(),
            Vault {
                status: VaultStatus::Cancelled,
                ..Vault::default()
            },
        );

        let label = Some("My new vault".to_string());

        let err =
            update_vault_handler(deps.as_mut(), mock_info(USER, &[]), vault.id, label.clone())
                .unwrap_err();

        assert_eq!(err.to_string(), "Error: vault is already cancelled");
    }

    #[test]
    fn updates_the_vault_label() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let label = Some("123456789".repeat(10).to_string());

        update_vault_handler(deps.as_mut(), mock_info(USER, &[]), vault.id, label.clone()).unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(updated_vault.label, label);
    }
}
