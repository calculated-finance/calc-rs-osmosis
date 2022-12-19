use crate::{
    error::ContractError,
    state::{
        cache::CACHE,
        triggers::delete_trigger,
        vaults::{get_vault, update_vault},
    },
    types::vault::Vault,
};
use base::{helpers::message_helpers::get_attribute_in_event, vaults::vault::VaultStatus};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Reply, Response};
use cosmwasm_std::{CosmosMsg, StdError, StdResult, SubMsgResult, Uint128};

pub fn after_fin_limit_order_withdrawn_for_cancel_vault(
    deps: DepsMut,
    _env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id.into())?;
    match reply.result {
        SubMsgResult::Ok(_) => {
            let withdraw_order_response = reply.result.into_result().unwrap();

            let received_amount =
                get_attribute_in_event(&withdraw_order_response.events, "transfer", "amount")?
                    .trim_end_matches(&vault.get_receive_denom().to_string())
                    .parse::<Uint128>()
                    .expect("limit order withdrawn amount");

            let coin_received = Coin {
                denom: vault.get_receive_denom().clone(),
                amount: received_amount,
            };

            let mut response = Response::new()
                .add_attribute("method", "fin_limit_order_withdrawn_for_cancel_vault");

            if coin_received.amount.gt(&Uint128::zero()) {
                response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![coin_received],
                }));
            }

            update_vault(
                deps.storage,
                vault.id.into(),
                |existing_vault| -> StdResult<Vault> {
                    match existing_vault {
                        Some(mut existing_vault) => {
                            existing_vault.status = VaultStatus::Cancelled;
                            existing_vault.balance = Coin::new(0, existing_vault.get_swap_denom());
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

            delete_trigger(deps.storage, vault.id.into())?;

            Ok(response)
        }
        SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to withdraw fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}
