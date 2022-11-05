use crate::contract::AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID;
use crate::error::ContractError;
use crate::state::cache::{CACHE, LIMIT_ORDER_CACHE};
use crate::state::triggers::delete_trigger;
use crate::state::vaults::{get_vault, update_vault};
use crate::vault::Vault;
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
                    .expect("returned amount should be a valid u128");

            // if the entire amount isnt retracted, order was partially filled need to send the partially filled assets to user
            if amount_retracted != limit_order_cache.original_offer_amount {
                let unswapped_balance = Coin {
                    denom: vault.get_swap_denom().clone(),
                    amount: vault.balance.amount - vault.get_swap_amount().amount
                        + amount_retracted,
                };

                if unswapped_balance.amount.gt(&Uint128::zero()) {
                    response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                        to_address: vault.owner.to_string(),
                        amount: vec![unswapped_balance.clone()],
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
