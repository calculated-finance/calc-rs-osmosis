use crate::error::ContractError;
use crate::state::{save_event, vault_store};
use crate::validation_helpers::{assert_denom_matches_pair_denom, assert_exactly_one_asset};
use base::events::event::{EventBuilder, EventData};

use base::vaults::vault::{Vault, VaultConfiguration, VaultStatus};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, MessageInfo, Response, Uint128};

pub fn deposit(
    deps: DepsMut,
    info: MessageInfo,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    let vault = vault_store().load(deps.storage, vault_id.into())?;
    if info.sender != vault.owner {
        return Err(ContractError::Unauthorized {});
    }

    match vault.configuration {
        VaultConfiguration::DCA {
            pair,
            swap_amount: _,
            position_type,
            slippage_tolerance: _,
        } => {
            assert_exactly_one_asset(info.funds.clone())?;
            assert_denom_matches_pair_denom(
                pair.clone(),
                info.funds.clone(),
                position_type.clone(),
            )?;

            vault_store().update(
                deps.storage,
                vault.id.into(),
                |existing_vault| -> Result<Vault, ContractError> {
                    match existing_vault {
                        Some(mut existing_vault) => {
                            existing_vault.balances[0].amount += info.funds[0].amount;
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

            save_event(
                deps.storage,
                EventBuilder::new(
                    vault.owner,
                    vault.id,
                    EventData::FundsDepositedToVault {
                        amount: info.funds[0].clone(),
                    },
                ),
            )?;

            Ok(Response::new().add_attribute("method", "deposit"))
        }
    }
}
