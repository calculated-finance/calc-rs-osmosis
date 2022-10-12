use crate::{
    error::ContractError,
    state::{trigger_store, vault_store, CACHE, LIMIT_ORDER_CACHE},
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Reply, Response};

pub fn fin_limit_order_withdrawn_for_cancel_vault(
    deps: DepsMut,
    _env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = vault_store().load(deps.storage, cache.vault_id.into())?;
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;

            let trigger_store = trigger_store();

            let fin_limit_order_trigger =
                trigger_store.load(deps.storage, vault.trigger_id.unwrap().into())?;

            // send assets from partially filled order to owner
            let filled_amount = Coin {
                denom: vault.get_receive_denom().clone(),
                amount: limit_order_cache.filled,
            };

            let filled_amount_bank_msg = BankMsg::Send {
                to_address: vault.owner.to_string(),
                amount: vec![filled_amount.clone()],
            };

            trigger_store.remove(deps.storage, fin_limit_order_trigger.id.u128())?;

            vault_store().remove(deps.storage, vault.id.into())?;

            LIMIT_ORDER_CACHE.remove(deps.storage);
            CACHE.remove(deps.storage);

            Ok(Response::new()
                .add_attribute("method", "after_cancel_trigger_withdraw_order")
                .add_message(filled_amount_bank_msg))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to withdraw fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}
