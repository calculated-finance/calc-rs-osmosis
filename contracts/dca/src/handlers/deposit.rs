use crate::error::ContractError;
use crate::helpers::validation_helpers::{
    assert_contract_is_not_paused, assert_deposited_denom_matches_send_denom,
    assert_exactly_one_asset, assert_vault_is_not_cancelled,
};
use crate::helpers::vault_helpers::get_dca_plus_model_id;
use crate::msg::ExecuteMsg;
use crate::state::events::create_event;
use crate::state::triggers::save_trigger;
use crate::state::vaults::{get_vault, update_vault};
use crate::types::dca_plus_config::DcaPlusConfig;
use crate::types::event::{EventBuilder, EventData};
use crate::types::trigger::{Trigger, TriggerConfiguration};
use crate::types::vault::VaultStatus;
use base::helpers::coin_helpers::add_to_coin;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Env, WasmMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, MessageInfo, Response, Uint128};

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: Addr,
    vault_id: Uint128,
) -> Result<Response, ContractError> {
    assert_contract_is_not_paused(deps.storage)?;
    deps.api.addr_validate(address.as_str())?;
    assert_exactly_one_asset(info.funds.clone())?;

    let mut vault = get_vault(deps.storage, vault_id.into())?;
    let vault_was_inactive = vault.is_inactive();

    if address != vault.owner {
        return Err(ContractError::CustomError {
            val: format!(
                "provided an incorrect owner address for vault id={:?}",
                vault_id
            ),
        });
    }

    assert_vault_is_not_cancelled(&vault)?;
    assert_deposited_denom_matches_send_denom(
        info.funds[0].denom.clone(),
        vault.clone().balance.denom,
    )?;

    vault.balance.amount += info.funds[0].amount;

    if !vault.is_scheduled() && vault.has_sufficient_funds() {
        vault.status = VaultStatus::Active
    }

    vault.dca_plus_config = vault
        .dca_plus_config
        .clone()
        .map(|dca_plus_config| DcaPlusConfig {
            total_deposit: add_to_coin(dca_plus_config.total_deposit, info.funds[0].amount),
            model_id: get_dca_plus_model_id(
                &env.block.time,
                &vault.balance,
                &vault.swap_amount,
                &vault.time_interval,
            ),
            ..dca_plus_config
        });

    update_vault(deps.storage, &vault)?;

    create_event(
        deps.storage,
        EventBuilder::new(
            vault.id,
            env.block.clone(),
            EventData::DcaVaultFundsDeposited {
                amount: info.funds[0].clone(),
            },
        ),
    )?;

    let mut response = Response::new().add_attribute("method", "deposit");

    if vault.is_active() && vault_was_inactive && vault.trigger.is_none() {
        save_trigger(
            deps.storage,
            Trigger {
                vault_id,
                configuration: TriggerConfiguration::Time {
                    target_time: env.block.time.clone(),
                },
            },
        )?;

        response = response.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::ExecuteTrigger {
                trigger_id: vault.id,
            })
            .unwrap(),
            funds: vec![],
        }));
    };

    Ok(response)
}
