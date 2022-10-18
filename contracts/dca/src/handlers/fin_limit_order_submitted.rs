use crate::error::ContractError;
use crate::state::{save_trigger, CACHE};
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::triggers::trigger::{Trigger, TriggerConfiguration};
use cosmwasm_std::Decimal256;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{DepsMut, Reply, Response, Uint128};
use std::str::FromStr;

pub fn fin_limit_order_submitted(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    match reply.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            let fin_submit_order_response = reply.result.into_result().unwrap();

            let order_idx = Uint128::from_str(
                &get_flat_map_for_event_type(&fin_submit_order_response.events, "wasm").unwrap()
                    ["order_idx"],
            )
            .unwrap();

            let cache = CACHE.load(deps.storage)?;

            save_trigger(
                deps.storage,
                Trigger {
                    vault_id: cache.vault_id,
                    configuration: TriggerConfiguration::FINLimitOrder {
                        order_idx: Some(order_idx),
                        target_price: Decimal256::one(),
                    },
                },
            )?;

            CACHE.remove(deps.storage);

            Ok(Response::new()
                .add_attribute("method", "fin_limit_order_submitted")
                .add_attribute("order_idx", order_idx))
        }
        cosmwasm_std::SubMsgResult::Err(e) => Err(ContractError::CustomError {
            val: format!("failed to create vault with fin limit order trigger: {}", e),
        }),
    }
}
