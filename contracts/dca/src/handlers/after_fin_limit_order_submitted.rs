use crate::error::ContractError;
use crate::state::cache::CACHE;
use crate::state::triggers::{get_trigger, save_trigger};
use base::helpers::message_helpers::get_attribute_in_event;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Reply, Response, Uint128};

pub fn after_fin_limit_order_submitted(
    deps: DepsMut,
    reply: Reply,
) -> Result<Response, ContractError> {
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_submit_order_response = reply.result.into_result().unwrap();

            let order_idx =
                get_attribute_in_event(&fin_submit_order_response.events, "wasm", "order_idx")?
                    .parse::<Uint128>()
                    .expect("returned order_idx should be a valid Uint128");

            let cache = CACHE.load(deps.storage)?;

            let trigger = get_trigger(deps.storage, cache.vault_id)?
                .expect(format!("fin limit order trigger for vault {:?}", cache.vault_id).as_str());

            match trigger.configuration {
                TriggerConfiguration::FinLimitOrder { target_price, .. } => {
                    save_trigger(
                        deps.storage,
                        Trigger {
                            vault_id: cache.vault_id,
                            configuration: TriggerConfiguration::FinLimitOrder {
                                order_idx: Some(order_idx),
                                target_price,
                            },
                        },
                    )?;
                }
                _ => panic!("should be a fin limit order trigger"),
            }

            Ok(Response::new()
                .add_attribute("method", "fin_limit_order_submitted")
                .add_attribute("order_idx", order_idx))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!("failed to create vault with fin limit order trigger: {}", e),
        }),
    }
}
