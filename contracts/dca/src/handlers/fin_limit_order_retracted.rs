use crate::contract::FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_ID;
use crate::error::ContractError;
use crate::state::{create_event, remove_trigger, vault_store, CACHE, LIMIT_ORDER_CACHE};
use base::events::event::{EventBuilder, EventData};
use base::helpers::message_helpers::{find_first_attribute_by_key, find_first_event_by_type};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, Reply, Response, Uint128};
use fin_helpers::limit_orders::create_withdraw_limit_order_sub_msg;

pub fn fin_limit_order_retracted(
    deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response, ContractError> {
    let cache = CACHE.load(deps.storage)?;
    let vault = vault_store().load(deps.storage, cache.vault_id.into())?;

    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let limit_order_cache = LIMIT_ORDER_CACHE.load(deps.storage)?;

            let fin_retract_order_response = reply.result.into_result().unwrap();

            let wasm_trade_event =
                find_first_event_by_type(&fin_retract_order_response.events, "wasm").unwrap();

            // if this parse method works look to refactor
            let amount_retracted =
                find_first_attribute_by_key(&wasm_trade_event.attributes, "amount")
                    .unwrap()
                    .value
                    .parse::<Uint128>()
                    .unwrap();

            create_event(
                deps.storage,
                EventBuilder::new(vault.id, env.block, EventData::DCAVaultCancelled),
            )?;

            // if the entire amount isnt retracted, order was partially filled need to send the partially filled assets to user
            if amount_retracted != limit_order_cache.original_offer_amount {
                let retracted_balance = Coin {
                    denom: vault.get_swap_denom().clone(),
                    amount: vault.balance.amount - (vault.swap_amount - amount_retracted),
                };

                let retracted_amount_bank_msg = BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![retracted_balance.clone()],
                };

                let fin_withdraw_sub_msg = create_withdraw_limit_order_sub_msg(
                    vault.pair.address.clone(),
                    vault.id,
                    FIN_LIMIT_ORDER_WITHDRAWN_FOR_CANCEL_VAULT_ID,
                );

                Ok(Response::new()
                    .add_attribute("method", "after_retract_order")
                    .add_attribute("withdraw_required", "true")
                    .add_submessage(fin_withdraw_sub_msg)
                    .add_message(retracted_amount_bank_msg))
            } else {
                let bank_msg = BankMsg::Send {
                    to_address: vault.owner.to_string(),
                    amount: vec![vault.balance.clone()],
                };

                vault_store().remove(deps.storage, vault.id.into())?;
                remove_trigger(deps.storage, vault.id.into())?;

                LIMIT_ORDER_CACHE.remove(deps.storage);
                CACHE.remove(deps.storage);

                Ok(Response::new()
                    .add_attribute("method", "after_retract_order")
                    .add_attribute("withdraw_required", "false")
                    .add_message(bank_msg))
            }
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!(
                "failed to retract fin limit order for vault id: {} - {}",
                vault.id, e
            ),
        }),
    }
}
