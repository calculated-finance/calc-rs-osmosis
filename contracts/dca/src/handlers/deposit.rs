use crate::error::ContractError;
use crate::state::{create_event, vault_store};
use crate::validation_helpers::{assert_denom_matches_pair_denom, assert_exactly_one_asset};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData};

use base::vaults::vault::VaultStatus;
use cosmwasm_std::Env;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, MessageInfo, Response, Uint128};

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let validated_address = deps.api.addr_validate(address.as_str())?;
    let vault = vault_store().load(deps.storage, vault_id.into())?;

    if validated_address != vault.owner {
        return Err(ContractError::Unauthorized {});
    }

    assert_exactly_one_asset(info.funds.clone())?;
    assert_denom_matches_pair_denom(
        vault.pair.clone(),
        info.funds.clone(),
        vault.position_type.clone(),
    )?;

    vault_store().update(
        deps.storage,
        vault.id.into(),
        |existing_vault| -> Result<Vault, ContractError> {
            match existing_vault {
                Some(mut existing_vault) => {
                    existing_vault.balance.amount += info.funds[0].amount;
                    if !existing_vault.low_funds() {
                        existing_vault.status = VaultStatus::Active
                    }
                    Ok(existing_vault)
                }
                None => Err(ContractError::CustomError {
                    val: format!(
                        "could not find vault for address: {} with id: {}",
                        vault.owner.clone(),
                        vault.id
                    ),
                }),
            }
        },
    )?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block,
            EventData::FundsDepositedToDCAVault {
                amount: info.funds[0].clone(),
            },
        ),
    )?;

    Ok(Response::new().add_attribute("method", "deposit"))
}
