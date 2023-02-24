use crate::contract::AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID;
use crate::error::ContractError;
use crate::state::cache::{CACHE, LIMIT_ORDER_CACHE};
use crate::state::triggers::delete_trigger;
use crate::state::vaults::{get_vault, update_vault};
use crate::types::vault::Vault;
use base::vaults::vault::VaultStatus;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Reply, Response, Uint128};
use cosmwasm_std::{CosmosMsg, StdError, StdResult, SubMsgResult};
use fin_helpers::limit_orders::create_withdraw_limit_order_sub_msg;

pub fn after_fin_limit_order_retracted(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id)?;
    let mut response = Response::new().add_attribute("method", "fin_limit_order_retracted");

    match reply.result {
        SubMsgResult::Ok(_) => {
            let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;

            let swap_denom_balance = &deps
                .querier
                .query_balance(&env.contract.address, &vault.get_swap_denom())?;

            let amount_retracted = swap_denom_balance
                .amount
                .checked_sub(limit_order_cache.swap_denom_balance.amount)
                .expect("amount retracted");

            if amount_retracted != limit_order_cache.original_offer_amount {
                let swap_denom_to_return = Coin {
                    denom: vault.get_swap_denom().clone(),
                    amount: vault.balance.amount + amount_retracted,
                };

                if swap_denom_to_return.amount.gt(&Uint128::zero()) {
                    response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                        to_address: vault.owner.to_string(),
                        amount: vec![swap_denom_to_return.clone()],
                    }));
                }

                // if the entire amount isnt retracted order was partially filled,
                // we need to withdraw and send the partially filled assets to user
                let fin_withdraw_sub_msg = create_withdraw_limit_order_sub_msg(
                    vault.pair.address.clone(),
                    limit_order_cache.order_idx,
                    AFTER_FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_REPLY_ID,
                );

                Ok(response
                    .add_attribute("withdraw_required", "true")
                    .add_submessage(fin_withdraw_sub_msg))
            } else {
                response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![Coin::new(
                        (amount_retracted + vault.balance.amount).into(),
                        vault.get_swap_denom(),
                    )],
                }));

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
