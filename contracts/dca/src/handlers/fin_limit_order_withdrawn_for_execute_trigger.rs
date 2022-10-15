use crate::dca_configuration::DCAConfiguration;
use crate::error::ContractError;
use crate::state::{
    save_event, trigger_store, vault_store, Config, CACHE, CONFIG, LIMIT_ORDER_CACHE,
    TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID,
};
use base::events::event::{EventBuilder, EventData};
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use base::vaults::vault::{Vault, VaultStatus};
use cosmwasm_std::Env;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Reply, Response, StdResult, Uint128};

pub fn fin_limit_order_withdrawn_for_execute_vault(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;
    let vault = vault_store().load(deps.storage, cache.vault_id.into())?;
    let trigger_store = trigger_store();
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_limit_order_trigger =
                trigger_store.load(deps.storage, vault.trigger_id.unwrap().into())?;

            trigger_store.remove(deps.storage, fin_limit_order_trigger.id.u128())?;

            let config = CONFIG.update(deps.storage, |mut config| -> StdResult<Config> {
                config.trigger_count = config.trigger_count.checked_add(Uint128::new(1))?;
                Ok(config)
            })?;

            let time_trigger_configuration = TIME_TRIGGER_CONFIGURATIONS_BY_VAULT_ID
                .load(deps.storage, vault.id.into())?
                .into_time()
                .unwrap();

            let time_trigger = Trigger {
                id: config.trigger_count,
                owner: vault.owner.clone(),
                vault_id: vault.id,
                configuration: TriggerConfiguration::Time {
                    time_interval: time_trigger_configuration.0,
                    target_time: time_trigger_configuration.1,
                },
            };

            trigger_store.save(deps.storage, time_trigger.id.u128(), &time_trigger)?;

            vault_store().update(
                deps.storage,
                vault.id.into(),
                |vault| -> Result<Vault<DCAConfiguration>, ContractError> {
                    match vault {
                        Some(mut existing_vault) => {
                            existing_vault.configuration.balance.amount -=
                                existing_vault.configuration.get_swap_amount().amount;
                            existing_vault.trigger_id = Some(time_trigger.id);

                            if existing_vault.configuration.low_funds() {
                                existing_vault.status = VaultStatus::Inactive
                            }

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

            let coin_received = Coin {
                denom: vault.configuration.get_receive_denom().clone(),
                amount: limit_order_cache.filled,
            };

            let config = CONFIG.load(deps.storage)?;

            let execution_fee = Coin::new(
                (coin_received.amount * config.fee_rate).u128(),
                &coin_received.denom,
            );

            let funds_to_redistribute = Coin::new(
                (coin_received.amount - execution_fee.amount).u128(),
                &coin_received.denom,
            );

            let vault_owner_bank_msg: BankMsg = BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![funds_to_redistribute],
            };

            let fee_collector_bank_msg: BankMsg = BankMsg::Send {
                to_address: config.fee_collector.to_string(),
                amount: vec![execution_fee.clone()],
            };

            save_event(
                deps.storage,
                EventBuilder::new(
                    vault.id,
                    env.block,
                    EventData::DCAVaultExecutionCompleted {
                        sent: Coin {
                            denom: vault.configuration.get_swap_denom().clone(),
                            amount: limit_order_cache.original_offer_amount,
                        },
                        received: coin_received,
                        fee: execution_fee,
                    },
                ),
            )?;

            LIMIT_ORDER_CACHE.remove(deps.storage);
            CACHE.remove(deps.storage);

            Ok(Response::new()
                .add_attribute("method", "after_withdraw_order")
                .add_attribute("trigger_id", time_trigger.id)
                .add_message(vault_owner_bank_msg)
                .add_message(fee_collector_bank_msg))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to withdraw fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}
