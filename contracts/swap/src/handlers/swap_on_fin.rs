use crate::{
    contract::{ContractResult, AFTER_FIN_SWAP_REPLY_ID},
    state::cache::{SwapCache, SWAP_CACHE},
    types::callback::Callback,
    validation::{assert_exactly_one_asset, assert_sender_is_contract},
};
use base::pair::Pair;
use cosmwasm_std::{
    Binary, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, ReplyOn, Response, WasmMsg,
};
use fin_helpers::swaps::create_fin_swap_message;

pub fn swap_on_fin_handler(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    pair: Pair,
    slippage_tolerance: Option<Decimal>,
    callback: Binary,
) -> ContractResult<Response> {
    assert_sender_is_contract(&info.sender, env)?;
    assert_exactly_one_asset(&info.funds)?;

    let swap_amount = info.funds[0].clone();

    let receive_denom_balance = deps.querier.query_balance(
        &env.contract.address,
        match pair.base_denom == swap_amount.denom {
            true => pair.quote_denom.clone(),
            false => pair.base_denom.clone(),
        },
    )?;

    SWAP_CACHE.save(
        deps.storage,
        &SwapCache {
            callback: Callback {
                address: info.sender.clone(),
                msg: callback,
            },
            receive_denom_balance,
        },
    )?;

    Ok(Response::new().add_submessage(create_fin_swap_message(
        deps.querier,
        pair,
        swap_amount,
        slippage_tolerance,
        Some(AFTER_FIN_SWAP_REPLY_ID),
        Some(ReplyOn::Success),
    )?))
}

pub fn after_swap_on_fin_handler(deps: DepsMut, env: Env) -> ContractResult<Response> {
    let swap_cache = SWAP_CACHE.load(deps.storage)?;

    let receive_denom_balance = deps.querier.query_balance(
        &env.contract.address,
        &swap_cache.receive_denom_balance.denom,
    )?;

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: swap_cache.callback.address.to_string(),
            msg: swap_cache.callback.msg,
            funds: vec![receive_denom_balance],
        })),
    )
}

#[cfg(test)]
mod swap_on_fin_tests {
    use base::pair::Pair;
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        to_binary, Addr, Coin, CosmosMsg, SubMsg, WasmMsg,
    };
    use kujira::fin::ExecuteMsg;

    use crate::{
        contract::AFTER_FIN_SWAP_REPLY_ID,
        handlers::swap_on_fin::swap_on_fin_handler,
        state::cache::{SwapCache, SWAP_CACHE},
        types::callback::Callback,
    };

    #[test]
    fn saves_current_receive_denom_balance_to_cache() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(
            &env.contract.address.to_string(),
            &[Coin::new(10000, "base-denom")],
        );

        let pair = Pair {
            address: Addr::unchecked("pair-addr"),
            base_denom: "base-denom".to_string(),
            quote_denom: "quote-denom".to_string(),
        };

        let callback = to_binary(&"test").unwrap();

        swap_on_fin_handler(deps.as_mut(), &env, &info, pair, None, callback.clone()).unwrap();

        let swap_cache = SWAP_CACHE.load(&deps.storage).unwrap();

        assert_eq!(
            swap_cache,
            SwapCache {
                callback: Callback {
                    address: info.sender,
                    msg: callback,
                },
                receive_denom_balance: Coin::new(0, "quote-denom"),
            }
        );
    }

    #[test]
    fn sends_fin_swap_message() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let swap_amount = Coin::new(10000, "base-denom");
        let info = mock_info(&env.contract.address.to_string(), &[swap_amount.clone()]);

        let pair = Pair {
            address: Addr::unchecked("pair-addr"),
            base_denom: "base-denom".to_string(),
            quote_denom: "quote-denom".to_string(),
        };

        let callback = to_binary(&"test").unwrap();

        let response = swap_on_fin_handler(
            deps.as_mut(),
            &env,
            &info,
            pair.clone(),
            None,
            callback.clone(),
        )
        .unwrap();

        assert!(response.messages.contains(&SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair.address.to_string(),
                msg: to_binary(&ExecuteMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    offer_asset: None,
                    to: None,
                })
                .unwrap(),
                funds: vec![swap_amount],
            }),
            AFTER_FIN_SWAP_REPLY_ID
        )));
    }
}

#[cfg(test)]
mod after_swap_on_fin_tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        to_binary, Coin, CosmosMsg, SubMsg, WasmMsg,
    };

    use crate::state::cache::{SwapCache, SWAP_CACHE};

    use super::after_swap_on_fin_handler;

    #[test]
    fn sends_callback_message() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        SWAP_CACHE
            .save(
                deps.as_mut().storage,
                &SwapCache {
                    callback: crate::types::callback::Callback {
                        address: env.contract.address.clone(),
                        msg: to_binary("test").unwrap(),
                    },
                    receive_denom_balance: Coin::new(0, "denom"),
                },
            )
            .unwrap();

        let received_amount = Coin::new(10000, "denom");

        deps.querier
            .update_balance(env.contract.address.clone(), vec![received_amount.clone()]);

        let result = after_swap_on_fin_handler(deps.as_mut(), env.clone()).unwrap();

        assert!(result
            .messages
            .contains(&SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary("test").unwrap(),
                funds: vec![received_amount],
            }))));
    }
}
