use crate::{
    msg::ExecuteMsg,
    shared::helpers::get_cheapest_swap_path,
    state::swap_messages::{get_next_swap_id, save_swap_messages},
    types::{callback::Callback, pair::Pair},
    validation::assert_exactly_one_asset,
};
use base::pair::Pair as FinPair;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Decimal256, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};
use std::collections::VecDeque;

pub fn create_swap_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    target_denom: String,
    slippage_tolerance: Option<Decimal256>,
    on_complete: Option<Callback>,
) -> StdResult<Response> {
    assert_exactly_one_asset(&info.funds)?;

    let cheapest_swap_path = get_cheapest_swap_path(deps.as_ref(), &info.funds[0], &target_denom)?;

    let swap_id = get_next_swap_id(deps.storage)?;
    let on_continue = to_binary(&ExecuteMsg::ContinueSwap { swap_id })?;

    let mut swap_messages = cheapest_swap_path
        .pairs
        .iter()
        .map(|pair| {
            generate_swap_message(
                env.clone(),
                pair.clone(),
                slippage_tolerance,
                on_continue.clone(),
            )
        })
        .flatten()
        .collect::<VecDeque<Callback>>();

    swap_messages.push_back(on_complete.unwrap_or(Callback {
        address: env.contract.address.clone(),
        msg: to_binary(&ExecuteMsg::SendFunds {
            address: info.sender,
        })?,
    }));

    save_swap_messages(deps.storage, swap_id, swap_messages)?;

    Ok(Response::new()
        .add_attribute("method", "swap")
        .add_attribute("swap_id", swap_id.to_string())
        .add_attribute(
            "path",
            format!(
                "[{}]",
                cheapest_swap_path
                    .pairs
                    .iter()
                    .map(|p| format!("{:?}", p))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        )
        .add_attribute("estimated_price", cheapest_swap_path.price.to_string())
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: on_continue,
            funds: info.funds,
        })))
}

fn generate_swap_message(
    env: Env,
    pair: Pair,
    slippage_tolerance: Option<Decimal256>,
    callback: Binary,
) -> StdResult<Callback> {
    match pair {
        Pair::Fin {
            address,
            base_denom,
            quote_denom,
        } => Ok(Callback {
            address: env.contract.address,
            msg: to_binary(&ExecuteMsg::SwapOnFin {
                pair: FinPair {
                    address,
                    base_denom,
                    quote_denom,
                },
                slippage_tolerance,
                callback,
            })?,
        }),
    }
}

#[cfg(test)]
mod swap_tests {
    use super::*;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        Coin, Uint128,
    };

    #[test]
    fn swap_with_no_swap_asset_should_fail() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &[]);

        let response = create_swap_handler(
            deps.as_mut(),
            env,
            info,
            "target_denom".to_string(),
            None,
            None,
        );

        assert_eq!(
            response.unwrap_err().to_string(),
            "Generic error: received 0 denoms but required exactly 1"
        )
    }

    #[test]
    fn swap_with_no_path_should_fail() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(
            "admin",
            &[Coin {
                denom: "swap_denom".to_string(),
                amount: Uint128::new(1000000),
            }],
        );

        let response = create_swap_handler(
            deps.as_mut(),
            env,
            info,
            "target_denom".to_string(),
            None,
            None,
        );

        assert_eq!(
            response.unwrap_err().to_string(),
            "Generic error: no path found between swap_denom and target_denom"
        )
    }
}
