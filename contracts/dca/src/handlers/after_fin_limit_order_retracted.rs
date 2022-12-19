use crate::contract::AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID;
use crate::error::ContractError;
use crate::state::cache::{CACHE, LIMIT_ORDER_CACHE};
use crate::state::fin_limit_order_change_timestamp::FIN_LIMIT_ORDER_CHANGE_TIMESTAMP;
use crate::state::triggers::delete_trigger;
use crate::state::vaults::{get_vault, update_vault};
use crate::types::vault::Vault;
use base::helpers::message_helpers::get_attribute_in_event;
use base::vaults::vault::VaultStatus;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Reply, Response, Uint128};
use cosmwasm_std::{CosmosMsg, StdError, StdResult, SubMsgResult};
use fin_helpers::limit_orders::create_withdraw_limit_order_sub_msg;

pub fn after_fin_limit_order_retracted(
    deps: DepsMut,
    _env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id)?;
    let mut response = Response::new().add_attribute("method", "fin_limit_order_retracted");

    match reply.result {
        SubMsgResult::Ok(_) => {
            let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;

            let fin_retract_order_response = reply.result.into_result().unwrap();

            let amount_retracted =
                get_attribute_in_event(&fin_retract_order_response.events, "wasm", "amount")?
                    .parse::<Uint128>()
                    .expect("limit order retracted amount");

            let fin_limit_order_change_timestamp =
                FIN_LIMIT_ORDER_CHANGE_TIMESTAMP.may_load(deps.storage)?;

            let is_new_fin_limit_order = fin_limit_order_change_timestamp.is_some()
                && limit_order_cache.created_at > fin_limit_order_change_timestamp.unwrap();

            // if the entire amount isnt retracted, order was partially filled need to send the partially filled assets to user
            if amount_retracted != limit_order_cache.original_offer_amount {
                let swap_denom_to_return = Coin {
                    denom: vault.get_swap_denom().clone(),
                    amount: if is_new_fin_limit_order {
                        vault.balance.amount + amount_retracted
                    } else {
                        vault.balance.amount - vault.get_swap_amount().amount + amount_retracted
                    },
                };

                if swap_denom_to_return.amount.gt(&Uint128::zero()) {
                    response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                        to_address: vault.owner.to_string(),
                        amount: vec![swap_denom_to_return.clone()],
                    }));
                }

                let fin_withdraw_sub_msg = create_withdraw_limit_order_sub_msg(
                    vault.pair.address.clone(),
                    limit_order_cache.order_idx,
                    AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
                );

                Ok(response
                    .add_attribute("withdraw_required", "true")
                    .add_submessage(fin_withdraw_sub_msg))
            } else {
                if is_new_fin_limit_order {
                    response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                        to_address: vault.owner.to_string(),
                        amount: vec![Coin::new(amount_retracted.into(), vault.get_swap_denom())],
                    }));
                }

                if vault.balance.amount.gt(&Uint128::zero()) {
                    response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                        to_address: vault.owner.to_string(),
                        amount: vec![vault.balance.clone()],
                    }));
                }

                update_vault(
                    deps.storage,
                    vault.id.into(),
                    |existing_vault| -> StdResult<Vault> {
                        match existing_vault {
                            Some(mut existing_vault) => {
                                existing_vault.status = VaultStatus::Cancelled;
                                existing_vault.balance =
                                    Coin::new(0, existing_vault.get_swap_denom());
                                Ok(existing_vault)
                            }
                            None => Err(StdError::NotFound {
                                kind: format!(
                                    "vault for address: {} with id: {}",
                                    vault.owner, vault.id
                                ),
                            }),
                        }
                    },
                )?;

                delete_trigger(deps.storage, vault.id)?;

                Ok(response.add_attribute("withdraw_required", "false"))
            }
        }
        SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to retract fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}
