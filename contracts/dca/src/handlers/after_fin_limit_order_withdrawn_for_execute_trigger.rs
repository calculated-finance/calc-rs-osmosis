use crate::contract::AFTER_FIN_SWAP_REPLY_ID;
use crate::error::ContractError;
use crate::helpers::vault_helpers::get_swap_amount;
use crate::state::cache::{SwapCache, CACHE, LIMIT_ORDER_CACHE, SWAP_CACHE};
use crate::state::vaults::get_vault;
use cosmwasm_std::{BankMsg, CosmosMsg, Env, ReplyOn};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{Coin, DepsMut, Reply, Response};
use fin_helpers::swaps::create_fin_swap_message;

pub fn after_fin_limit_order_withdrawn_for_execute_vault(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = get_vault(deps.storage, cache.vault_id.into())?;

    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
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

            SWAP_CACHE.save(
                deps.storage,
                &SwapCache {
                    swap_denom_balance: deps
                        .querier
                        .query_balance(&env.contract.address, &vault.get_swap_denom())?,
                    receive_denom_balance: Coin::new(
                        (deps
                            .querier
                            .query_balance(&env.contract.address, &vault.get_receive_denom())?
                            .amount
                            - withdrawn_amount)
                            .into(),
                        vault.get_receive_denom().clone(),
                    ),
                },
            )?;

            Ok(Response::new()
                .add_attribute("method", "fin_limit_order_withdrawn_for_execute_vault")
                .add_attribute("vault_id", vault.id)
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![coin_received],
                }))
                .add_submessage(create_fin_swap_message(
                    deps.querier,
                    vault.pair.clone(),
                    get_swap_amount(&deps.as_ref(), &env, vault.clone())?,
                    vault.slippage_tolerance,
                    Some(AFTER_FIN_SWAP_REPLY_ID),
                    Some(ReplyOn::Always),
                )?))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to withdraw fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}
