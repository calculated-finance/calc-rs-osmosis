use crate::msg::{FinBookResponse, FinPoolResponseWithoutDenom};
use cosmwasm_std::{
    from_binary,
    testing::{MockApi, MockQuerier},
    to_binary, ContractResult, Decimal, MemoryStorage, OwnedDeps, SystemResult, Uint128, WasmQuery,
};
use kujira::fin::QueryMsg;

pub fn set_fin_price(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, MockQuerier>,
    price: &'static Decimal,
    offer_size: &'static Uint128,
    depth: &'static Uint128,
) {
    deps.querier.update_wasm(|query| match query.clone() {
        WasmQuery::Smart { msg, .. } => match from_binary(&msg).unwrap() {
            QueryMsg::Book { offset, .. } => SystemResult::Ok(ContractResult::Ok(
                to_binary(&FinBookResponse {
                    base: match offset {
                        Some(0) | None => (0..depth.u128())
                            .map(|order| FinPoolResponseWithoutDenom {
                                quote_price: price.clone()
                                    + Decimal::percent(order.try_into().unwrap()),
                                total_offer_amount: offer_size.clone(),
                            })
                            .collect(),
                        _ => vec![],
                    },
                    quote: match offset {
                        Some(0) | None => (0..depth.u128())
                            .map(|order| FinPoolResponseWithoutDenom {
                                quote_price: price.clone()
                                    - Decimal::percent(order.try_into().unwrap()),
                                total_offer_amount: offer_size.clone(),
                            })
                            .collect(),
                        _ => vec![],
                    },
                })
                .unwrap(),
            )),
            _ => panic!(),
        },
        _ => panic!(),
    });
}
