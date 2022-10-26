use crate::error::ContractError;
use crate::state::{create_event, get_vault, update_vault};
use crate::validation_helpers::{
    assert_denom_matches_pair_denom, assert_exactly_one_asset, assert_vault_is_not_cancelled,
};
use crate::vault::Vault;
use base::events::event::{EventBuilder, EventData};

use base::vaults::vault::VaultStatus;
use cosmwasm_std::{Addr, Env, StdError, StdResult};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, MessageInfo, Response, Uint128};

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: Addr,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(address.as_str())?;
    let vault = get_vault(deps.storage, vault_id.into())?;

    if address != vault.owner {
        return Err(ContractError::Unauthorized {});
    }

    assert_vault_is_not_cancelled(&vault)?;
    assert_exactly_one_asset(info.funds.clone())?;
    assert_denom_matches_pair_denom(
        vault.pair.clone(),
        info.funds.clone(),
        vault.position_type.clone(),
    )?;

    update_vault(
        deps.storage,
        vault.id.into(),
        |existing_vault| -> StdResult<Vault> {
            match existing_vault {
                Some(mut existing_vault) => {
                    existing_vault.balance.amount += info.funds[0].amount;
                    if !existing_vault.low_funds() {
                        existing_vault.status = VaultStatus::Active
                    }
                    Ok(existing_vault)
                }
                None => Err(StdError::NotFound {
                    kind: format!(
                        "vault for address: {} with id: {}",
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
            EventData::DCAVaultFundsDeposited {
                amount: info.funds[0].clone(),
            },
        ),
    )?;

    Ok(Response::new().add_attribute("method", "deposit"))
}
