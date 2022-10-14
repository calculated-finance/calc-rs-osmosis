use crate::dca_configuration::DCAConfiguration;
use crate::error::ContractError;
use crate::state::{
    trigger_store, vault_store, Config, CACHE, CONFIG, FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID,
};
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use base::vaults::vault::Vault;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Reply, Response, StdResult, Uint128};
use std::str::FromStr;

pub fn fin_limit_order_submitted(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_submit_order_response = reply.result.into_result().unwrap();

            let order_idx = Uint128::from_str(
                &get_flat_map_for_event_type(&fin_submit_order_response.events, "wasm").unwrap()
                    ["order_idx"],
            )
            .unwrap();

            let cache = CACHE.load(deps.storage)?;

            let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
                config.trigger_count = config.trigger_count.checked_add(Uint128::new(1))?;
                Ok(config)
            })?;

            let fin_limit_order_configuration_values = FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID
                .load(deps.storage, cache.vault_id.u128())?
                .into_fin_limit_order()
                .unwrap();

            let fin_limit_order_trigger = Trigger {
                id: config.trigger_count,
                owner: cache.owner.clone(),
                vault_id: cache.vault_id,
                configuration: TriggerConfiguration::FINLimitOrder {
                    target_price: fin_limit_order_configuration_values.0,
                    order_idx: Some(order_idx),
                },
            };

            vault_store().update(
                deps.storage,
                cache.vault_id.into(),
                |vault| -> Result<Vault<DCAConfiguration>, ContractError> {
                    match vault {
                        Some(mut existing_vault) => {
                            existing_vault.trigger_id = Some(fin_limit_order_trigger.id);
                            Ok(existing_vault)
                        }
                        None => Err(ContractError::CustomError {
                            val: format!(
                                "could not find vault for address: {} with id: {}",
                                cache.owner, cache.vault_id
                            ),
                        }),
                    }
                },
            )?;

            trigger_store().save(
                deps.storage,
                fin_limit_order_trigger.id.u128(),
                &fin_limit_order_trigger,
            )?;

            FIN_LIMIT_ORDER_CONFIGURATIONS_BY_VAULT_ID.remove(deps.storage, cache.vault_id.u128());

            CACHE.remove(deps.storage);

            Ok(Response::new()
                .add_attribute("method", "after_submit_order")
                .add_attribute("trigger_id", fin_limit_order_trigger.id))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!("failed to create vault with fin limit order trigger: {}", e),
        }),
    }
}
