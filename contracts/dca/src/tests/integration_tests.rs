use base::triggers::time_configuration::{TimeConfiguration, TimeInterval};
use base::vaults::dca_vault::PositionType;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Decimal256, Empty, Event, Response, StdResult, Uint128,
    Uint256, Uint64,
};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use kujira::denom::Denom;
use kujira::fin::{
    BookResponse, ExecuteMsg as FINExecuteMsg, InstantiateMsg as FINInstantiateMsg, OrderResponse,
    PoolResponse, QueryMsg as FINQueryMsg,
};
use std::str::FromStr;

use crate::contract::reply;
use crate::msg::{
    ExecuteMsg, ExecutionsResponse, InstantiateMsg, MigrateMsg, QueryMsg, TriggerIdsResponse,
    TriggersResponse, VaultsResponse,
};

const USER: &str = "user";
const ADMIN: &str = "admin";
const DENOM_UKUJI: &str = "ukuji";
const DENOM_UTEST: &str = "utest";

trait AppHelpers {
    fn add_liquidity(&mut self, address: Addr, coins: Vec<Coin>);
    fn elapse_time(&mut self, seconds: u64);
}

impl AppHelpers for App {
    fn add_liquidity(&mut self, address: Addr, coins: Vec<Coin>) {
        self.init_modules(|router, _, storage| {
            router.bank.init_balance(storage, &address, coins).unwrap();
        });
    }

    fn elapse_time(&mut self, seconds: u64) {
        self.update_block(|mut block_info| {
            block_info.time = block_info.time.plus_seconds(seconds);
            let seconds_per_block = 5u64;
            block_info.height += seconds / seconds_per_block;
        });
    }
}

fn mock_app() -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(ADMIN),
                vec![Coin {
                    denom: String::from("bitcoin-lol"),
                    amount: Uint128::new(1000000),
                }],
            )
            .unwrap();
    })
}

fn create_calc_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(reply);

    Box::new(contract)
}

fn create_mock_book_response(
    quote_denom: String,
    base_price: Decimal256,
    quote_price: Decimal256,
) -> BookResponse {
    let pool_response_quote = PoolResponse {
        quote_price: quote_price,
        offer_denom: Denom::from(quote_denom.clone()),
        total_offer_amount: Uint256::zero(),
    };

    let pool_response_base = PoolResponse {
        quote_price: base_price,
        offer_denom: Denom::from(quote_denom),
        total_offer_amount: Uint256::zero(),
    };

    let book_response = BookResponse {
        base: vec![pool_response_base.clone()],
        quote: vec![pool_response_quote.clone()],
    };
    book_response
}

fn create_mock_fin_contract_success() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        |_, _, info, msg: FINExecuteMsg| -> StdResult<Response> {
            match msg {
                FINExecuteMsg::Swap {
                    belief_price: _,
                    max_spread: _,
                    to: _,
                    offer_asset: _,
                } => {
                    let received_coin = info.funds[0].clone();
                    let coin_to_send = match received_coin.denom.as_str() {
                        DENOM_UKUJI => Coin {
                            denom: String::from(DENOM_UTEST),
                            amount: info.funds[0].amount,
                        },
                        DENOM_UTEST => Coin {
                            denom: String::from(DENOM_UKUJI),
                            amount: info.funds[0].amount,
                        },
                        _ => Coin {
                            denom: String::from(DENOM_UTEST),
                            amount: info.funds[0].amount,
                        },
                    };
                    let event = Event::new("trade")
                        .add_attribute("market", "value")
                        .add_attribute("base_amount", received_coin.amount.clone())
                        .add_attribute("quote_amount", received_coin.amount.clone());
                    Ok(Response::new().add_event(event).add_message(BankMsg::Send {
                        to_address: info.sender.to_string(),
                        amount: vec![coin_to_send],
                    }))
                }
                FINExecuteMsg::SubmitOrder { price } => {
                    Ok(Response::new().add_attribute("order_idx", "1"))
                }
                FINExecuteMsg::WithdrawOrders { order_idxs } => Ok(Response::default()),
                _ => Ok(Response::default()),
            }
        },
        |_, _, _, _: FINInstantiateMsg| -> StdResult<Response> { Ok(Response::new()) },
        |_, env, msg: FINQueryMsg| -> StdResult<Binary> {
            match msg {
                FINQueryMsg::Book {
                    limit: _,
                    offset: _,
                } => Ok(to_binary(&create_mock_book_response(
                    String::from(DENOM_UTEST),
                    Decimal256::from_str("10")?,
                    Decimal256::from_str("10")?,
                ))?),
                FINQueryMsg::Order { order_idx } => {
                    let response = OrderResponse {
                        created_at: env.block.time,
                        owner: Addr::unchecked(USER),
                        idx: Uint128::new(1),
                        quote_price: Decimal256::from_str("1.0").unwrap(),
                        original_offer_amount: Uint256::from_str("100").unwrap(),
                        filled_amount: Uint256::from_str("100").unwrap(),
                        offer_denom: Denom::from(DENOM_UKUJI),
                        offer_amount: Uint256::zero(),
                    };
                    Ok(to_binary(&response)?)
                }
                _ => {
                    #[derive(Serialize)]
                    pub struct Mock;
                    Ok(to_binary(&Mock)?)
                }
            }
        },
    );
    Box::new(contract)
}

fn create_mock_fin_contract_success_higher_price() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        |_, _, info, msg: FINExecuteMsg| -> StdResult<Response> {
            match msg {
                FINExecuteMsg::Swap {
                    belief_price: _,
                    max_spread: _,
                    to: _,
                    offer_asset: _,
                } => {
                    let received_coin = info.funds[0].clone();
                    let coin_to_send = match received_coin.denom.as_str() {
                        DENOM_UKUJI => Coin {
                            denom: String::from(DENOM_UTEST),
                            amount: info.funds[0].amount,
                        },
                        DENOM_UTEST => Coin {
                            denom: String::from(DENOM_UKUJI),
                            amount: info.funds[0].amount,
                        },
                        _ => Coin {
                            denom: String::from(DENOM_UTEST),
                            amount: info.funds[0].amount,
                        },
                    };
                    let event = Event::new("trade")
                        .add_attribute("market", "value")
                        .add_attribute("base_amount", received_coin.amount.clone())
                        .add_attribute("quote_amount", received_coin.amount.clone());
                    Ok(Response::new().add_event(event).add_message(BankMsg::Send {
                        to_address: info.sender.to_string(),
                        amount: vec![coin_to_send],
                    }))
                }
                _ => Ok(Response::default()),
            }
        },
        |_, _, _, _: FINInstantiateMsg| -> StdResult<Response> { Ok(Response::new()) },
        |_, _, msg: FINQueryMsg| -> StdResult<Binary> {
            match msg {
                FINQueryMsg::Book {
                    limit: _,
                    offset: _,
                } => Ok(to_binary(&create_mock_book_response(
                    String::from(DENOM_UTEST),
                    Decimal256::from_str("12")?,
                    Decimal256::from_str("12")?,
                ))?),
                _ => {
                    #[derive(Serialize)]
                    pub struct Mock;
                    Ok(to_binary(&Mock)?)
                }
            }
        },
    );
    Box::new(contract)
}

fn create_mock_fin_contract_fail_slippage_tolerance() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        |_, _, _, msg: FINExecuteMsg| -> StdResult<Response> {
            match msg {
                FINExecuteMsg::Swap {
                    belief_price: _,
                    max_spread: _,
                    offer_asset: _,
                    to: _,
                } => Err(cosmwasm_std::StdError::GenericErr {
                    msg: String::from("Max spread exceeded 0.992445703493862134"),
                }),
                _ => Ok(Response::default()),
            }
        },
        |_, _, _, _: FINInstantiateMsg| -> StdResult<Response> { Ok(Response::new()) },
        |_, _, msg: FINQueryMsg| -> StdResult<Binary> {
            match msg {
                FINQueryMsg::Book {
                    limit: _,
                    offset: _,
                } => Ok(to_binary(&create_mock_book_response(
                    String::from(DENOM_UTEST),
                    Decimal256::from_str("10")?,
                    Decimal256::from_str("10")?,
                ))?),
                _ => {
                    #[derive(Serialize)]
                    pub struct Mock;
                    Ok(to_binary(&Mock)?)
                }
            }
        },
    );
    Box::new(contract)
}

#[test]
fn create_vault_with_price_trigger_should_succeed() {
    let mut app = mock_app();
    let calc_code_id = app.store_code(create_calc_contract());
    let calc_init_message = InstantiateMsg {
        admin: String::from(ADMIN),
    };
    let calc_contract_address = app
        .instantiate_contract(
            calc_code_id,
            Addr::unchecked(ADMIN),
            &calc_init_message,
            &[],
            "calc-dca",
            None,
        )
        .unwrap();

    let fin_code_id = app.store_code(create_mock_fin_contract_success());
    let denoms: [Denom; 2] = [Denom::from(DENOM_UTEST), Denom::from(DENOM_UKUJI)];

    let fin_init_message = FINInstantiateMsg {
        decimal_delta: None,
        denoms,
        owner: Addr::unchecked(ADMIN),
        price_precision: kujira::precision::Precision::DecimalPlaces(3),
    };

    let fin_contract_address = app
        .instantiate_contract(
            fin_code_id,
            Addr::unchecked(ADMIN),
            &fin_init_message,
            &[],
            "fin",
            None,
        )
        .unwrap();

    app.add_liquidity(
        Addr::unchecked(ADMIN),
        vec![
            Coin {
                denom: String::from(DENOM_UTEST),
                amount: Uint128::new(200),
            },
            Coin {
                denom: String::from(DENOM_UKUJI),
                amount: Uint128::new(200),
            },
        ],
    );

    app.add_liquidity(
        Addr::unchecked(USER),
        vec![
            Coin {
                denom: String::from(DENOM_UTEST),
                amount: Uint128::new(0),
            },
            Coin {
                denom: String::from(DENOM_UKUJI),
                amount: Uint128::new(200),
            },
        ],
    );

    app.add_liquidity(
        calc_contract_address.clone(),
        vec![
            Coin {
                denom: String::from(DENOM_UTEST),
                amount: Uint128::new(200),
            },
            Coin {
                denom: String::from(DENOM_UKUJI),
                amount: Uint128::new(200),
            },
        ],
    );

    app.add_liquidity(
        fin_contract_address.clone(),
        vec![
            Coin {
                denom: String::from(DENOM_UTEST),
                amount: Uint128::new(200),
            },
            Coin {
                denom: String::from(DENOM_UKUJI),
                amount: Uint128::new(200),
            },
        ],
    );

    let create_pair_execute_message = ExecuteMsg::CreatePair {
        address: fin_contract_address.to_string(),
        base_denom: String::from(DENOM_UTEST),
        quote_denom: String::from(DENOM_UKUJI),
    };

    let _ = app
        .execute_contract(
            Addr::unchecked(ADMIN),
            calc_contract_address.clone(),
            &create_pair_execute_message,
            &[],
        )
        .unwrap();

    let create_vault_with_price_trigger = ExecuteMsg::CreateVaultWithFINLimitOrderTrigger {
        pair_address: fin_contract_address.to_string(),
        position_type: PositionType::Enter,
        slippage_tolerance: None,
        swap_amount: Uint128::new(100),
        total_executions: 2u16,
        time_interval: TimeInterval::Hourly,
        target_price: Decimal256::from_str("1.0").unwrap(),
    };

    let funds = vec![Coin {
        denom: String::from(DENOM_UKUJI),
        amount: Uint128::new(200),
    }];

    let result = app
        .execute_contract(
            Addr::unchecked(USER),
            calc_contract_address.clone(),
            &create_vault_with_price_trigger,
            &funds,
        )
        .unwrap();

    assert_eq!(
        result.events[1].attributes[1].value,
        "create_vault_with_fin_limit_order_trigger"
    );

    assert_eq!(result.events[1].attributes[2].value, "1");

    assert_eq!(result.events[1].attributes[3].value, "user");

    assert_eq!(result.events[1].attributes[4].value, "1");

    assert_eq!(result.events[5].attributes[1].value, "after_submit_order");

    assert_eq!(result.events[5].attributes[2].key, "trigger_id");

    assert_eq!(result.events[5].attributes[2].value, "1");

    let execute_fin_limit_order_trigger_by_order_idx =
        ExecuteMsg::ExecuteFINLimitOrderTriggerByOrderIdx {
            order_idx: Uint128::new(1),
        };

    let res = app
        .execute_contract(
            Addr::unchecked(ADMIN),
            calc_contract_address,
            &execute_fin_limit_order_trigger_by_order_idx,
            &[],
        )
        .unwrap();

    assert_eq!(
        res.events[1].attributes[1].value,
        "execute_fin_limit_order_trigger_by_order_idx"
    );

    assert_eq!(res.events[4].attributes[1].value, "after_withdraw_order");

    assert_eq!(res.events[4].attributes[2].key, "trigger_id");

    assert_eq!(res.events[4].attributes[2].value, "2");
    let balance_user = app
        .wrap()
        .query_balance(Addr::unchecked(USER), DENOM_UTEST)
        .unwrap();

    // let get_all_active_vaults_query_message = QueryMsg::GetAllActiveVaults {};

    // let get_all_active_vaults_response: VaultsResponse = app
    //     .wrap()
    //     .query_wasm_smart(
    //         calc_contract_address.clone(),
    //         &get_all_active_vaults_query_message,
    //     )
    //     .unwrap();

    assert_eq!(balance_user.amount, Uint128::new(100));

    // assert_eq!(balance_fin.amount, Uint128::new(100));

    // assert_eq!(get_all_active_vaults_response.vaults.len(), 1);
}

// #[test]
// fn execute_price_trigger_by_id_should_succeed() {
//     let mut app = mock_app();
//     let calc_code_id = app.store_code(create_calc_contract());
//     let calc_init_message = InstantiateMsg {
//         admin: String::from(ADMIN),
//     };
//     let calc_contract_address = app
//         .instantiate_contract(
//             calc_code_id,
//             Addr::unchecked(ADMIN),
//             &calc_init_message,
//             &[],
//             "calc-dca",
//             None,
//         )
//         .unwrap();

//     let fin_code_id = app.store_code(create_mock_fin_contract_success());
//     let denoms: [Denom; 2] = [Denom::from(DENOM_UTEST), Denom::from(DENOM_UKUJI)];

//     let fin_init_message = FINInstantiateMsg {
//         decimal_delta: None,
//         denoms,
//         owner: Addr::unchecked(ADMIN),
//         price_precision: kujira::precision::Precision::DecimalPlaces(3),
//     };

//     let fin_contract_address = app
//         .instantiate_contract(
//             fin_code_id,
//             Addr::unchecked(ADMIN),
//             &fin_init_message,
//             &[],
//             "fin",
//             Some(String::from(ADMIN)),
//         )
//         .unwrap();

//     app.add_liquidity(
//         Addr::unchecked(USER),
//         Coin {
//             denom: String::from(DENOM_UTEST),
//             amount: Uint128::new(100),
//         },
//     );

//     app.add_liquidity(
//         fin_contract_address.clone(),
//         Coin {
//             denom: String::from(DENOM_UKUJI),
//             amount: Uint128::new(100),
//         },
//     );

//     let create_pair_execute_message = ExecuteMsg::CreatePair {
//         address: fin_contract_address.to_string(),
//         base_denom: String::from(DENOM_UKUJI),
//         quote_denom: String::from(DENOM_UTEST),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &create_pair_execute_message,
//             &[],
//         )
//         .unwrap();

//     let create_vault_with_price_trigger = ExecuteMsg::CreateVaultWithFINLimitOrderTrigger {
//         pair_address: fin_contract_address.to_string(),
//         position_type: PositionType::Enter,
//         slippage_tolerance: None,
//         swap_amount: Uint128::new(100),
//         total_executions: 1u16,
//         time_interval: TimeInterval::Hourly,
//         target_price: Decimal256::from_str("5.0").unwrap(),
//     };

//     let funds = vec![Coin {
//         denom: String::from(DENOM_UTEST),
//         amount: Uint128::new(100),
//     }];

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(USER),
//             calc_contract_address.clone(),
//             &create_vault_with_price_trigger,
//             &funds,
//         )
//         .unwrap();

//     let msg = MigrateMsg {
//         admin: String::from(ADMIN),
//     };

//     let fin_code_id_new_response = app.store_code(create_mock_fin_contract_success_higher_price());

//     let migrate = app
//         .migrate_contract(
//             Addr::unchecked(ADMIN),
//             fin_contract_address,
//             &msg,
//             fin_code_id_new_response,
//         )
//         .unwrap();

//     let execute_price_trigger_by_id = ExecuteMsg::ExecutePriceTriggerById {
//         trigger_id: Uint128::new(1),
//     };

//     let result = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address,
//             &execute_price_trigger_by_id,
//             &[],
//         )
//         .unwrap();

//     assert_eq!(
//         result.events[0].attributes[1].value,
//         "execute price trigger by id"
//     )

//     // let balance_user = app
//     //     .wrap()
//     //     .query_balance(Addr::unchecked(USER), DENOM_UKUJI)
//     //     .unwrap();

//     // let balance_fin = app
//     //     .wrap()
//     //     .query_balance(fin_contract_address.clone(), DENOM_UKUJI)
//     //     .unwrap();

//     // let get_all_active_vaults_query_message = QueryMsg::GetAllActiveVaults {};

//     // let get_all_active_vaults_response: VaultsResponse = app
//     //     .wrap()
//     //     .query_wasm_smart(
//     //         calc_contract_address.clone(),
//     //         &get_all_active_vaults_query_message,
//     //     )
//     //     .unwrap();

//     // let get_all_price_triggers_by_address_and_price =
//     //     QueryMsg::GetAllPriceTriggersByAddressAndPrice {
//     //         address: fin_contract_address.to_string(),
//     //         price: Decimal256::from_str("6").unwrap(),
//     //     };

//     // let get_all_price_triggers_by_address_and_price: TriggerIdsResponse = app
//     //     .wrap()
//     //     .query_wasm_smart(
//     //         calc_contract_address,
//     //         &get_all_price_triggers_by_address_and_price,
//     //     )
//     //     .unwrap();

//     // assert_eq!(balance_user.amount, Uint128::new(0));

//     // assert_eq!(balance_fin.amount, Uint128::new(100));

//     // assert_eq!(get_all_active_vaults_response.vaults.len(), 1);

//     // assert_eq!(
//     //     get_all_price_triggers_by_address_and_price
//     //         .trigger_ids
//     //         .len(),
//     //     1
//     // );
// }

// #[test]
// fn execute_price_trigger_by_id_early_should_fail() {
//     let mut app = mock_app();
//     let calc_code_id = app.store_code(create_calc_contract());
//     let calc_init_message = InstantiateMsg {
//         admin: String::from(ADMIN),
//     };
//     let calc_contract_address = app
//         .instantiate_contract(
//             calc_code_id,
//             Addr::unchecked(ADMIN),
//             &calc_init_message,
//             &[],
//             "calc-dca",
//             None,
//         )
//         .unwrap();

//     let fin_code_id = app.store_code(create_mock_fin_contract_success());
//     let denoms: [Denom; 2] = [Denom::from(DENOM_UTEST), Denom::from(DENOM_UKUJI)];

//     let fin_init_message = FINInstantiateMsg {
//         decimal_delta: None,
//         denoms,
//         owner: Addr::unchecked(ADMIN),
//         price_precision: kujira::precision::Precision::DecimalPlaces(3),
//     };

//     let fin_contract_address = app
//         .instantiate_contract(
//             fin_code_id,
//             Addr::unchecked(ADMIN),
//             &fin_init_message,
//             &[],
//             "fin",
//             None,
//         )
//         .unwrap();

//     app.add_liquidity(
//         Addr::unchecked(USER),
//         Coin {
//             denom: String::from(DENOM_UTEST),
//             amount: Uint128::new(100),
//         },
//     );

//     app.add_liquidity(
//         fin_contract_address.clone(),
//         Coin {
//             denom: String::from(DENOM_UKUJI),
//             amount: Uint128::new(100),
//         },
//     );

//     let create_pair_execute_message = ExecuteMsg::CreatePair {
//         address: fin_contract_address.to_string(),
//         base_denom: String::from(DENOM_UKUJI),
//         quote_denom: String::from(DENOM_UTEST),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &create_pair_execute_message,
//             &[],
//         )
//         .unwrap();

//     let create_vault_with_price_trigger = ExecuteMsg::CreateVaultWithFINLimitOrderTrigger {
//         pair_address: fin_contract_address.to_string(),
//         position_type: PositionType::Enter,
//         slippage_tolerance: None,
//         swap_amount: Uint128::new(100),
//         total_executions: 1u16,
//         time_interval: TimeInterval::Hourly,
//         target_price: Decimal256::from_str("5.0").unwrap(),
//     };

//     let funds = vec![Coin {
//         denom: String::from(DENOM_UTEST),
//         amount: Uint128::new(100),
//     }];

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(USER),
//             calc_contract_address.clone(),
//             &create_vault_with_price_trigger,
//             &funds,
//         )
//         .unwrap();

//     let execute_price_trigger_by_id = ExecuteMsg::ExecutePriceTriggerById {
//         trigger_id: Uint128::new(1),
//     };

//     let result = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address,
//             &execute_price_trigger_by_id,
//             &[],
//         )
//         .unwrap_err();

//     assert_eq!(
//         result.root_cause().to_string(),
//         "Error: vault price target has not been hit yet. current price: 10, trigger price: 5"
//     )

//     // let balance_user = app
//     //     .wrap()
//     //     .query_balance(Addr::unchecked(USER), DENOM_UKUJI)
//     //     .unwrap();

//     // let balance_fin = app
//     //     .wrap()
//     //     .query_balance(fin_contract_address.clone(), DENOM_UKUJI)
//     //     .unwrap();

//     // let get_all_active_vaults_query_message = QueryMsg::GetAllActiveVaults {};

//     // let get_all_active_vaults_response: VaultsResponse = app
//     //     .wrap()
//     //     .query_wasm_smart(
//     //         calc_contract_address.clone(),
//     //         &get_all_active_vaults_query_message,
//     //     )
//     //     .unwrap();

//     // let get_all_price_triggers_by_address_and_price =
//     //     QueryMsg::GetAllPriceTriggersByAddressAndPrice {
//     //         address: fin_contract_address.to_string(),
//     //         price: Decimal256::from_str("6").unwrap(),
//     //     };

//     // let get_all_price_triggers_by_address_and_price: TriggerIdsResponse = app
//     //     .wrap()
//     //     .query_wasm_smart(
//     //         calc_contract_address,
//     //         &get_all_price_triggers_by_address_and_price,
//     //     )
//     //     .unwrap();

//     // assert_eq!(balance_user.amount, Uint128::new(0));

//     // assert_eq!(balance_fin.amount, Uint128::new(100));

//     // assert_eq!(get_all_active_vaults_response.vaults.len(), 1);

//     // assert_eq!(
//     //     get_all_price_triggers_by_address_and_price
//     //         .trigger_ids
//     //         .len(),
//     //     1
//     // );
// }

// #[test]
// fn execute_vault_by_address_and_id_should_succeed() {
//     let mut app = mock_app();
//     let calc_code_id = app.store_code(create_calc_contract());
//     let calc_init_message = InstantiateMsg {
//         admin: String::from(ADMIN),
//     };
//     let calc_contract_address = app
//         .instantiate_contract(
//             calc_code_id,
//             Addr::unchecked(ADMIN),
//             &calc_init_message,
//             &[],
//             "calc-dca",
//             None,
//         )
//         .unwrap();

//     let fin_code_id = app.store_code(create_mock_fin_contract_success());
//     let denoms: [Denom; 2] = [Denom::from(DENOM_UTEST), Denom::from(DENOM_UKUJI)];

//     let fin_init_message = FINInstantiateMsg {
//         decimal_delta: None,
//         denoms,
//         owner: Addr::unchecked(ADMIN),
//         price_precision: kujira::precision::Precision::DecimalPlaces(3),
//     };

//     let fin_contract_address = app
//         .instantiate_contract(
//             fin_code_id,
//             Addr::unchecked(ADMIN),
//             &fin_init_message,
//             &[],
//             "fin",
//             None,
//         )
//         .unwrap();

//     app.add_liquidity(
//         Addr::unchecked(USER),
//         Coin {
//             denom: String::from(DENOM_UTEST),
//             amount: Uint128::new(100),
//         },
//     );

//     app.add_liquidity(
//         fin_contract_address.clone(),
//         Coin {
//             denom: String::from(DENOM_UKUJI),
//             amount: Uint128::new(100),
//         },
//     );

//     let create_pair_execute_message = ExecuteMsg::CreatePair {
//         address: fin_contract_address.to_string(),
//         base_denom: String::from(DENOM_UKUJI),
//         quote_denom: String::from(DENOM_UTEST),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &create_pair_execute_message,
//             &[],
//         )
//         .unwrap();

//     let create_vault_by_address_and_id_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
//         pair_address: fin_contract_address.to_string(),
//         position_type: PositionType::Enter,
//         slippage_tolerance: None,
//         swap_amount: Uint128::new(100),
//         total_executions: 1u16,
//         time_interval: TimeInterval::Hourly,
//         target_start_time_utc_seconds: None,
//     };

//     let funds = vec![Coin {
//         denom: String::from(DENOM_UTEST),
//         amount: Uint128::new(100),
//     }];

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(USER),
//             calc_contract_address.clone(),
//             &create_vault_by_address_and_id_execute_message,
//             &funds,
//         )
//         .unwrap();

//     let execute_time_trigger_by_id_execute_message = ExecuteMsg::ExecuteTimeTriggerById {
//         trigger_id: Uint128::new(1),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &execute_time_trigger_by_id_execute_message,
//             &[],
//         )
//         .unwrap();

//     let balance_user = app
//         .wrap()
//         .query_balance(Addr::unchecked(USER), DENOM_UKUJI)
//         .unwrap();

//     let balance_fin = app
//         .wrap()
//         .query_balance(fin_contract_address, DENOM_UKUJI)
//         .unwrap();

//     let get_all_active_vaults_query_message = QueryMsg::GetAllActiveVaults {};

//     let get_all_active_vaults_response: VaultsResponse = app
//         .wrap()
//         .query_wasm_smart(
//             calc_contract_address.clone(),
//             &get_all_active_vaults_query_message,
//         )
//         .unwrap();

//     let get_all_executions_by_vault_id_query_message = QueryMsg::GetAllExecutionsByVaultId {
//         vault_id: Uint128::new(1),
//     };

//     let get_all_executions_by_vault_id_response: ExecutionsResponse = app
//         .wrap()
//         .query_wasm_smart(
//             calc_contract_address,
//             &get_all_executions_by_vault_id_query_message,
//         )
//         .unwrap();

//     assert_eq!(balance_user.amount, Uint128::new(100));

//     assert_eq!(balance_fin.amount, Uint128::new(0));

//     assert_eq!(get_all_active_vaults_response.vaults.len(), 0);

//     assert_eq!(get_all_executions_by_vault_id_response.executions.len(), 1);

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .vault_id,
//         Uint128::new(1)
//     );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_result,
//     //     ExecutionResult::Success
//     // );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .block_height,
//         Uint64::new(app.block_info().height)
//     );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .sequence_number,
//         1
//     );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_information.unwrap().sent_amount,
//     //     Uint128::new(100)
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_information.unwrap().sent_denom,
//     //     DENOM_UTEST
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_information.unwrap().received_amount,
//     //     Uint128::new(100)
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_information.unwrap().received_denom,
//     //     DENOM_UKUJI
//     // );
// }

// #[test]
// fn execute_vault_by_address_and_id_multiple_times_should_succeed() {
//     let mut app = mock_app();
//     let calc_code_id = app.store_code(create_calc_contract());
//     let calc_init_message = InstantiateMsg {
//         admin: String::from(ADMIN),
//     };
//     let calc_contract_address = app
//         .instantiate_contract(
//             calc_code_id,
//             Addr::unchecked(ADMIN),
//             &calc_init_message,
//             &[],
//             "calc-dca",
//             None,
//         )
//         .unwrap();

//     let fin_code_id = app.store_code(create_mock_fin_contract_success());
//     let denoms: [Denom; 2] = [Denom::from(DENOM_UTEST), Denom::from(DENOM_UKUJI)];

//     let fin_init_message = FINInstantiateMsg {
//         decimal_delta: None,
//         denoms,
//         owner: Addr::unchecked(ADMIN),
//         price_precision: kujira::precision::Precision::DecimalPlaces(3),
//     };
//     let fin_contract_address = app
//         .instantiate_contract(
//             fin_code_id,
//             Addr::unchecked(ADMIN),
//             &fin_init_message,
//             &[],
//             "fin",
//             None,
//         )
//         .unwrap();

//     app.add_liquidity(
//         Addr::unchecked(USER),
//         Coin {
//             denom: String::from(DENOM_UTEST),
//             amount: Uint128::new(200),
//         },
//     );

//     app.add_liquidity(
//         fin_contract_address.clone(),
//         Coin {
//             denom: String::from(DENOM_UKUJI),
//             amount: Uint128::new(200),
//         },
//     );

//     let create_pair_execute_message = ExecuteMsg::CreatePair {
//         address: fin_contract_address.to_string(),
//         base_denom: String::from(DENOM_UKUJI),
//         quote_denom: String::from(DENOM_UTEST),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &create_pair_execute_message,
//             &[],
//         )
//         .unwrap();

//     let create_vault_by_address_and_id_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
//         pair_address: fin_contract_address.to_string(),
//         position_type: PositionType::Enter,
//         slippage_tolerance: None,
//         swap_amount: Uint128::new(100),
//         total_executions: 2u16,
//         time_interval: TimeInterval::Hourly,
//         target_start_time_utc_seconds: None,
//     };

//     let funds = vec![Coin {
//         denom: String::from(DENOM_UTEST),
//         amount: Uint128::new(200),
//     }];

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(USER),
//             calc_contract_address.clone(),
//             &create_vault_by_address_and_id_execute_message,
//             &funds,
//         )
//         .unwrap();

//     let first_execute_time_trigger_by_id_execute_message = ExecuteMsg::ExecuteTimeTriggerById {
//         trigger_id: Uint128::new(1),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &first_execute_time_trigger_by_id_execute_message,
//             &[],
//         )
//         .unwrap();

//     let first_execution_block_info = app.block_info();

//     app.elapse_time(3600u64);

//     let second_execute_time_trigger_by_id_execute_message = ExecuteMsg::ExecuteTimeTriggerById {
//         trigger_id: Uint128::new(1),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &second_execute_time_trigger_by_id_execute_message,
//             &[],
//         )
//         .unwrap();

//     let balance_user = app
//         .wrap()
//         .query_balance(Addr::unchecked(USER), DENOM_UKUJI)
//         .unwrap();

//     let balance_fin = app
//         .wrap()
//         .query_balance(fin_contract_address, DENOM_UKUJI)
//         .unwrap();

//     let get_all_active_vaults_query_message = QueryMsg::GetAllActiveVaults {};

//     let get_all_active_vaults_response: VaultsResponse = app
//         .wrap()
//         .query_wasm_smart(
//             calc_contract_address.clone(),
//             &get_all_active_vaults_query_message,
//         )
//         .unwrap();

//     let get_all_executions_by_vault_id_query_message = QueryMsg::GetAllExecutionsByVaultId {
//         vault_id: Uint128::new(1),
//     };

//     let get_all_executions_by_vault_id_response: ExecutionsResponse = app
//         .wrap()
//         .query_wasm_smart(
//             calc_contract_address,
//             &get_all_executions_by_vault_id_query_message,
//         )
//         .unwrap();

//     assert_eq!(balance_user.amount, Uint128::new(200));

//     assert_eq!(balance_fin.amount, Uint128::new(0));

//     assert_eq!(get_all_active_vaults_response.vaults.len(), 0);

//     assert_eq!(get_all_executions_by_vault_id_response.executions.len(), 2);

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .vault_id,
//         Uint128::new(1)
//     );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[1]
//             .clone()
//             .vault_id,
//         Uint128::new(1)
//     );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_result,
//     //     ExecutionResult::Success
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[1].clone().execution_result,
//     //     ExecutionResult::Success
//     // );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .block_height,
//         Uint64::new(first_execution_block_info.height)
//     );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[1]
//             .clone()
//             .block_height,
//         Uint64::new(app.block_info().height)
//     );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .sequence_number,
//         1
//     );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[1]
//             .clone()
//             .sequence_number,
//         2
//     );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_information.unwrap().sent_amount,
//     //     Uint128::new(100)
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[1].clone().execution_information.unwrap().sent_amount,
//     //     Uint128::new(100)
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_information.unwrap().sent_denom,
//     //     DENOM_UTEST
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[1].clone().execution_information.unwrap().sent_denom,
//     //     DENOM_UTEST
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_information.unwrap().received_amount,
//     //     Uint128::new(100)
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[1].clone().execution_information.unwrap().received_amount,
//     //     Uint128::new(100)
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_information.unwrap().received_denom,
//     //     DENOM_UKUJI
//     // );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[1].clone().execution_information.unwrap().received_denom,
//     //     DENOM_UKUJI
//     // );
// }

// #[test]
// fn execute_vault_by_address_and_id_exceed_slippage_should_skip_execution() {
//     let mut app = mock_app();
//     let calc_code_id = app.store_code(create_calc_contract());
//     let calc_init_message = InstantiateMsg {
//         admin: String::from(ADMIN),
//     };
//     let calc_contract_address = app
//         .instantiate_contract(
//             calc_code_id,
//             Addr::unchecked(ADMIN),
//             &calc_init_message,
//             &[],
//             "calc-dca",
//             None,
//         )
//         .unwrap();

//     let fin_code_id = app.store_code(create_mock_fin_contract_fail_slippage_tolerance());
//     let denoms: [Denom; 2] = [Denom::from(DENOM_UTEST), Denom::from(DENOM_UKUJI)];

//     let fin_init_message = FINInstantiateMsg {
//         decimal_delta: None,
//         denoms,
//         owner: Addr::unchecked(ADMIN),
//         price_precision: kujira::precision::Precision::DecimalPlaces(3),
//     };
//     let fin_contract_address = app
//         .instantiate_contract(
//             fin_code_id,
//             Addr::unchecked(ADMIN),
//             &fin_init_message,
//             &[],
//             "fin",
//             None,
//         )
//         .unwrap();

//     app.add_liquidity(
//         Addr::unchecked(USER),
//         Coin {
//             denom: String::from(DENOM_UTEST),
//             amount: Uint128::new(100),
//         },
//     );

//     app.add_liquidity(
//         fin_contract_address.clone(),
//         Coin {
//             denom: String::from(DENOM_UKUJI),
//             amount: Uint128::new(100),
//         },
//     );

//     let create_pair_execute_message = ExecuteMsg::CreatePair {
//         address: fin_contract_address.to_string(),
//         base_denom: String::from(DENOM_UKUJI),
//         quote_denom: String::from(DENOM_UTEST),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &create_pair_execute_message,
//             &[],
//         )
//         .unwrap();

//     let slippage = "0.5";

//     let create_vault_by_address_and_id_execute_message = ExecuteMsg::CreateVaultWithTimeTrigger {
//         pair_address: fin_contract_address.to_string(),
//         position_type: PositionType::Enter,
//         slippage_tolerance: Some(Decimal256::from_str(&slippage).unwrap()),
//         swap_amount: Uint128::new(100),
//         total_executions: 1u16,
//         time_interval: TimeInterval::Hourly,
//         target_start_time_utc_seconds: None,
//     };

//     let funds = vec![Coin {
//         denom: String::from(DENOM_UTEST),
//         amount: Uint128::new(100),
//     }];

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(USER),
//             calc_contract_address.clone(),
//             &create_vault_by_address_and_id_execute_message,
//             &funds,
//         )
//         .unwrap();

//     let execute_vault_by_address_and_id_execute_message = ExecuteMsg::ExecuteTimeTriggerById {
//         trigger_id: Uint128::new(1),
//     };

//     let _ = app
//         .execute_contract(
//             Addr::unchecked(ADMIN),
//             calc_contract_address.clone(),
//             &execute_vault_by_address_and_id_execute_message,
//             &[],
//         )
//         .unwrap();

//     let balance_user = app
//         .wrap()
//         .query_balance(Addr::unchecked(USER), DENOM_UKUJI)
//         .unwrap();

//     let balance_fin = app
//         .wrap()
//         .query_balance(fin_contract_address, DENOM_UKUJI)
//         .unwrap();

//     let get_all_active_vaults_query_message = QueryMsg::GetAllActiveVaults {};

//     let get_all_active_vaults_response: VaultsResponse = app
//         .wrap()
//         .query_wasm_smart(
//             calc_contract_address.clone(),
//             &get_all_active_vaults_query_message,
//         )
//         .unwrap();

//     let get_all_time_triggers_query_message = QueryMsg::GetAllTimeTriggers {};

//     let get_all_time_triggers_response: TriggersResponse<TimeConfiguration> = app
//         .wrap()
//         .query_wasm_smart(
//             calc_contract_address.clone(),
//             &get_all_time_triggers_query_message,
//         )
//         .unwrap();

//     let get_all_executions_by_vault_id_query_message = QueryMsg::GetAllExecutionsByVaultId {
//         vault_id: Uint128::new(1),
//     };

//     let get_all_executions_by_vault_id_response: ExecutionsResponse = app
//         .wrap()
//         .query_wasm_smart(
//             calc_contract_address,
//             &get_all_executions_by_vault_id_query_message,
//         )
//         .unwrap();

//     assert_eq!(balance_user.amount, Uint128::new(0));

//     assert_eq!(balance_fin.amount, Uint128::new(100));

//     assert_eq!(
//         get_all_time_triggers_response.triggers[0]
//             .configuration
//             .triggers_remaining,
//         1
//     );

//     assert_eq!(get_all_active_vaults_response.vaults.len(), 1);

//     assert_eq!(get_all_executions_by_vault_id_response.executions.len(), 1);

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .vault_id,
//         Uint128::new(1)
//     );

//     // assert_eq!(
//     //     get_all_executions_by_vault_id_response.executions[0].clone().execution_result,
//     //     ExecutionResult::SlippageToleranceExceeded
//     // );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .block_height,
//         Uint64::new(app.block_info().height)
//     );

//     assert_eq!(
//         get_all_executions_by_vault_id_response.executions[0]
//             .clone()
//             .sequence_number,
//         1
//     );
// }
