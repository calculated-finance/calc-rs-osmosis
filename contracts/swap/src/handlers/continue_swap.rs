use crate::{
    state::swap_messages::{get_swap_messages, save_swap_messages},
    validation::assert_exactly_one_asset,
};
use cosmwasm_std::{CosmosMsg, DepsMut, MessageInfo, Response, StdResult, WasmMsg};

pub fn continue_swap_handler(
    deps: DepsMut,
    info: MessageInfo,
    swap_id: u64,
) -> StdResult<Response> {
    assert_exactly_one_asset(info.funds.clone())?;

    let mut swap_messages = get_swap_messages(deps.storage, swap_id)?;
    let next_swap_message = swap_messages.pop_front().expect("next callback");

    save_swap_messages(deps.storage, swap_id, swap_messages)?;

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: next_swap_message.address.to_string(),
            msg: next_swap_message.msg,
            funds: info.funds,
        })),
    )
}

#[cfg(test)]
mod continue_swap_tests {
    use super::continue_swap_handler;
    use crate::{
        msg::ExecuteMsg,
        state::swap_messages::{get_swap_messages, save_swap_messages},
        types::callback::Callback,
    };
    use base::pair::Pair;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary, Addr, Coin, CosmosMsg, SubMsg, WasmMsg,
    };
    use std::collections::VecDeque;

    #[test]
    fn removes_next_message_from_the_queue() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let mut messages = VecDeque::<Callback>::new();

        messages.push_front(Callback {
            address: Addr::unchecked("next-addr"),
            msg: to_binary(&ExecuteMsg::SwapOnFin {
                pair: Pair {
                    address: Addr::unchecked("pair-addr"),
                    base_denom: "base-denom".to_string(),
                    quote_denom: "quote-denom".to_string(),
                },
                slippage_tolerance: None,
                callback: to_binary(&"test").unwrap(),
            })
            .unwrap(),
        });

        messages.push_front(Callback {
            address: Addr::unchecked("next-addr"),
            msg: to_binary(&ExecuteMsg::SwapOnFin {
                pair: Pair {
                    address: Addr::unchecked("pair-addr"),
                    base_denom: "base-denom".to_string(),
                    quote_denom: "quote-denom".to_string(),
                },
                slippage_tolerance: None,
                callback: to_binary(&"test").unwrap(),
            })
            .unwrap(),
        });

        let swap_id = 1;

        save_swap_messages(deps.as_mut().storage, swap_id, messages).unwrap();

        continue_swap_handler(
            deps.as_mut(),
            mock_info(
                &env.contract.address.to_string(),
                &[Coin::new(10000, "base-denom")],
            ),
            swap_id,
        )
        .unwrap();

        let swap_messages = get_swap_messages(&deps.storage, swap_id).unwrap();
        assert_eq!(swap_messages.len(), 1);
    }

    #[test]
    fn invokes_next_message_with_funds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let mut messages = VecDeque::<Callback>::new();

        let final_message = Callback {
            address: Addr::unchecked("final-addr"),
            msg: to_binary(&ExecuteMsg::SwapOnFin {
                pair: Pair {
                    address: Addr::unchecked("pair-addr-1"),
                    base_denom: "base-denom".to_string(),
                    quote_denom: "quote-denom".to_string(),
                },
                slippage_tolerance: None,
                callback: to_binary(&"test").unwrap(),
            })
            .unwrap(),
        };

        let next_message = Callback {
            address: Addr::unchecked("next-addr"),
            msg: to_binary(&ExecuteMsg::SwapOnFin {
                pair: Pair {
                    address: Addr::unchecked("pair-addr-2"),
                    base_denom: "base-denom".to_string(),
                    quote_denom: "quote-denom".to_string(),
                },
                slippage_tolerance: None,
                callback: to_binary(&"test").unwrap(),
            })
            .unwrap(),
        };

        messages.push_front(final_message.clone());
        messages.push_front(next_message.clone());

        let swap_id = 1;

        save_swap_messages(deps.as_mut().storage, swap_id, messages).unwrap();

        let funds = Coin::new(10000, "base-denom");

        let result = continue_swap_handler(
            deps.as_mut(),
            mock_info(&env.contract.address.to_string(), &[funds.clone()]),
            swap_id,
        )
        .unwrap();

        assert!(result
            .messages
            .contains(&SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: next_message.address.to_string(),
                msg: next_message.msg,
                funds: vec![funds],
            }))))
    }
}
