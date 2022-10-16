use crate::dca_configuration::DCAConfiguration;
use crate::error::ContractError;
use crate::state::{save_event, vault_store};
use crate::validation_helpers::{assert_denom_matches_pair_denom, assert_exactly_one_asset};
use base::events::event::{EventBuilder, EventData};

use base::vaults::vault::{Vault, VaultStatus};
use cosmwasm_std::Env;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, MessageInfo, Response, Uint128};

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let vault = vault_store().load(deps.storage, vault_id.into())?;
    if info.sender != vault.owner {
        return Err(ContractError::Unauthorized {});
    }

    assert_exactly_one_asset(info.funds.clone())?;
    assert_denom_matches_pair_denom(
        vault.configuration.pair.clone(),
        info.funds.clone(),
        vault.configuration.position_type.clone(),
    )?;

    vault_store().update(
        deps.storage,
        vault.id.into(),
        |existing_vault| -> Result<Vault<DCAConfiguration>, ContractError> {
            match existing_vault {
                Some(mut existing_vault) => {
                    existing_vault.configuration.balance.amount += info.funds[0].amount;
                    if !existing_vault.configuration.low_funds() {
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

    save_event(
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
