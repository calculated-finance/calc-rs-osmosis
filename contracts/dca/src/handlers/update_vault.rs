use crate::{
    error::ContractError,
    helpers::validation::{
        assert_destination_allocations_add_up_to_one,
        assert_destination_callback_addresses_are_valid, assert_destinations_limit_is_not_breached,
        assert_label_is_no_longer_than_100_characters, assert_no_destination_allocations_are_zero,
        assert_vault_is_not_cancelled, asset_sender_is_vault_owner,
    },
    state::vaults::{get_vault, update_vault},
    types::destination::Destination,
};
use cosmwasm_std::{Decimal, DepsMut, MessageInfo, Response, Uint128};

pub fn update_vault_handler(
    deps: DepsMut,
    info: MessageInfo,
    vault_id: Uint128,
    label: Option<String>,
    destinations: Option<Vec<Destination>>,
) -> Result<Response, ContractError> {
    let mut vault = get_vault(deps.storage, vault_id)?;

    asset_sender_is_vault_owner(vault.owner.clone(), info.sender)?;
    assert_vault_is_not_cancelled(&vault)?;

    let mut response = Response::default()
        .add_attribute("update_vault", "true")
        .add_attribute("vault_id", vault.id);

    if let Some(label) = label {
        assert_label_is_no_longer_than_100_characters(&label)?;

        vault.label = Some(label.clone());
        response = response.add_attribute("label", label);
    }

    if let Some(mut destinations) = destinations {
        if destinations.is_empty() {
            destinations.push(Destination {
                allocation: Decimal::percent(100),
                address: vault.owner.clone(),
                msg: None,
            });
        }

        assert_destinations_limit_is_not_breached(&destinations)?;
        assert_destination_callback_addresses_are_valid(deps.as_ref(), &destinations)?;
        assert_no_destination_allocations_are_zero(&destinations)?;
        assert_destination_allocations_add_up_to_one(&destinations)?;

        vault.destinations = destinations.clone();
        response = response.add_attribute("destinations", format!("{:?}", destinations));
    }

    update_vault(deps.storage, &vault)?;

    Ok(response)
}

#[cfg(test)]
mod update_vault_tests {
    use super::update_vault_handler;
    use crate::{
        state::vaults::get_vault,
        tests::{
            helpers::{instantiate_contract, setup_vault},
            mocks::{ADMIN, USER},
        },
        types::{
            destination::Destination,
            vault::{Vault, VaultStatus},
        },
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Decimal,
    };

    #[test]
    fn with_label_longer_than_100_characters_fails() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let label = Some("12345678910".repeat(10).to_string());

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            label.clone(),
            None,
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: Vault label cannot be longer than 100 characters"
        );
    }

    #[test]
    fn for_vault_with_different_owner_fails() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]));

        let vault = setup_vault(
            deps.as_mut(),
            mock_env(),
            Vault {
                owner: Addr::unchecked("random"),
                ..Vault::default()
            },
        );

        let label = Some("My new vault".to_string());

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            label.clone(),
            None,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Unauthorized");
    }

    #[test]
    fn for_cancelled_vault_fails() {
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

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            label.clone(),
            None,
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "Error: vault is already cancelled");
    }

    #[test]
    fn with_more_than_10_destinations_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(10),
                msg: None,
            };
            11
        ];

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: no more than 10 destinations can be provided"
        );
    }

    #[test]
    fn with_destination_allocations_less_than_100_percent_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(10),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(10),
                msg: None,
            },
        ];

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: destination allocations must add up to 1"
        );
    }

    #[test]
    fn with_destination_allocations_more_than_100_percent_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(50),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(51),
                msg: None,
            },
        ];

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: destination allocations must add up to 1"
        );
    }

    #[test]
    fn with_destination_with_zero_allocation_fails() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(100),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::zero(),
                msg: None,
            },
        ];

        let err = update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations),
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Error: all destination allocations must be greater than 0"
        );
    }

    #[test]
    fn updates_the_vault_label() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let label = Some("123456789".repeat(10).to_string());

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            label.clone(),
            None,
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_eq!(updated_vault.label, label);
    }

    #[test]
    fn updates_the_vault_destinations() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        let destinations = vec![
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(50),
                msg: None,
            },
            Destination {
                address: Addr::unchecked("random"),
                allocation: Decimal::percent(50),
                msg: None,
            },
        ];

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(destinations.clone()),
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_ne!(updated_vault.destinations, vault.destinations);
        assert_eq!(updated_vault.destinations, destinations);
    }

    #[test]
    fn sets_the_vault_destination_to_owner_when_update_list_is_empty() {
        let mut deps = mock_dependencies();

        let vault = setup_vault(deps.as_mut(), mock_env(), Vault::default());

        update_vault_handler(
            deps.as_mut(),
            mock_info(USER, &[]),
            vault.id,
            None,
            Some(vec![]),
        )
        .unwrap();

        let updated_vault = get_vault(deps.as_ref().storage, vault.id).unwrap();

        assert_ne!(updated_vault.destinations, vault.destinations);
        assert_eq!(
            updated_vault.destinations,
            vec![Destination {
                address: vault.owner,
                allocation: Decimal::percent(100),
                msg: None,
            }]
        );
    }
}
