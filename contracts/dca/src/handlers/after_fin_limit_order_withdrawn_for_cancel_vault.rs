use crate::{
    error::ContractError,
    state::{
        cache::{CACHE, LIMIT_ORDER_CACHE},
        triggers::delete_trigger,
        vaults::{get_vault, update_vault},
    },
};
use base::vaults::vault::VaultStatus;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Reply, Response};
use cosmwasm_std::{CosmosMsg, SubMsgResult, Uint128};

pub fn after_fin_limit_order_withdrawn_for_cancel_vault(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let mut vault = get_vault(deps.storage, cache.vault_id.into())?;
    match reply.result {
        SubMsgResult::Ok(_) => {
            let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;

            let receive_denom_balance = &deps
                .querier
                .query_balance(&env.contract.address, &vault.get_receive_denom())?;

            let withdrawn_amount = receive_denom_balance
                .amount
                .checked_sub(limit_order_cache.receive_denom_balance.amount)
                .expect("withdrawn amount");

            let coin_received = Coin {
                denom: vault.get_receive_denom().clone(),
                amount: withdrawn_amount,
            };

            let mut response = Response::new()
                .add_attribute("method", "fin_limit_order_withdrawn_for_cancel_vault");

            if coin_received.amount.gt(&Uint128::zero()) {
                response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![coin_received],
                }));
            }

            vault.status = VaultStatus::Cancelled;
            vault.balance = Coin::new(0, vault.get_swap_denom());

            update_vault(deps.storage, &vault)?;

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
