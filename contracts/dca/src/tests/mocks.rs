use crate::constants::{ONE_THOUSAND, TWO_MICRONS};
use crate::contract::reply;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, VaultResponse};
use crate::state::config::FeeCollector;

use crate::types::vault::Vault;
use base::helpers::message_helpers::get_flat_map_for_event_type;
use base::triggers::trigger::TimeInterval;
use base::vaults::vault::Destination;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Decimal, Decimal256, Empty, Env, Event, MessageInfo,
    Response, StdResult, Uint128, Uint64,
};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use rand::Rng;
use std::collections::HashMap;
use std::str::FromStr;

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const FEE_COLLECTOR: &str = "fee_collector";
pub const DENOM_UOSMO: &str = "uosmo";
pub const DENOM_STAKE: &str = "stake";

pub struct MockApp {
    pub app: App,
    pub dca_contract_address: Addr,
    pub fin_contract_address: Addr,
    pub vault_ids: HashMap<String, Uint128>,
    pub fee_percent: Decimal,
}

impl MockApp {
    pub fn new(_fin_contract: Box<dyn Contract<Empty>>) -> Self {
        let mut app = AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(ADMIN),
                    vec![
                        Coin {
                            denom: String::from(DENOM_UOSMO),
                            amount: ONE_THOUSAND,
                        },
                        Coin {
                            denom: String::from(DENOM_STAKE),
                            amount: ONE_THOUSAND,
                        },
                    ],
                )
                .unwrap();
        });

        let dca_contract_address = Self::instantiate_contract(
            &mut app,
            Box::new(
                ContractWrapper::new(
                    crate::contract::execute,
                    crate::contract::instantiate,
                    crate::contract::query,
                )
                .with_reply(reply),
            ),
            Addr::unchecked(ADMIN),
            &InstantiateMsg {
                admin: Addr::unchecked(ADMIN),
                fee_collectors: vec![FeeCollector {
                    address: FEE_COLLECTOR.to_string(),
                    allocation: Decimal::from_str("1").unwrap(),
                }],
                swap_fee_percent: Decimal::from_str("0.0165").unwrap(),
                delegation_fee_percent: Decimal::from_str("0.0075").unwrap(),
                staking_router_address: Addr::unchecked("staking-router"),
                page_limit: 1000,
                paused: false,
                dca_plus_escrow_level: Decimal::from_str("0.05").unwrap(),
            },
            "dca",
        );

        app.init_modules(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &dca_contract_address,
                    vec![
                        Coin {
                            denom: String::from(DENOM_UOSMO),
                            amount: ONE_THOUSAND,
                        },
                        Coin {
                            denom: String::from(DENOM_STAKE),
                            amount: ONE_THOUSAND,
                        },
                    ],
                )
                .unwrap();
        });

        app.execute_contract(
            Addr::unchecked(ADMIN),
            dca_contract_address.clone(),
            &ExecuteMsg::CreatePool {
                pool_id: 0,
                base_denom: DENOM_STAKE.to_string(),
                quote_denom: DENOM_UOSMO.to_string(),
            },
            &[],
        )
        .unwrap();

        Self {
            app,
            dca_contract_address: dca_contract_address.clone(),
            fin_contract_address: dca_contract_address,
            vault_ids: HashMap::new(),
            fee_percent: Decimal::from_str("0.0165").unwrap(),
        }
    }

    fn instantiate_contract<T: Serialize>(
        app: &mut App,
        contract: Box<dyn Contract<Empty>>,
        sender: Addr,
        msg: &T,
        label: &str,
    ) -> Addr {
        let code_id = app.store_code(contract);
        let contract_address = app
            .instantiate_contract(code_id, sender, msg, &[], label, None)
            .unwrap();

        contract_address
    }

    pub fn with_funds_for(mut self, address: &Addr, amount: Uint128, denom: &str) -> MockApp {
        self.app
            .send_tokens(
                Addr::unchecked(ADMIN),
                address.clone(),
                &[Coin::new(amount.into(), denom.to_string())],
            )
            .unwrap();

        self
    }

    pub fn with_vault_with_unfilled_fin_limit_price_trigger(
        mut self,
        owner: &Addr,
        destinations: Option<Vec<Destination>>,
        balance: Coin,
        swap_amount: Uint128,
        label: &str,
    ) -> Self {
        let response = self
            .app
            .execute_contract(
                owner.clone(),
                self.dca_contract_address.clone(),
                &ExecuteMsg::CreateVault {
                    owner: None,
                    minimum_receive_amount: None,
                    label: Some("label".to_string()),
                    destinations,
                    pool_id: 0,
                    position_type: None,
                    slippage_tolerance: None,
                    swap_amount,
                    time_interval: TimeInterval::Hourly,
                    target_receive_amount: Some(swap_amount),
                    target_start_time_utc_seconds: None,
                    use_dca_plus: None,
                },
                &vec![balance],
            )
            .unwrap();

        self.vault_ids.insert(
            String::from(label),
            Uint128::from_str(
                &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
            )
            .unwrap(),
        );

        self
    }

    pub fn with_vault_with_filled_fin_limit_price_trigger(
        mut self,
        owner: &Addr,
        destinations: Option<Vec<Destination>>,
        balance: Coin,
        swap_amount: Uint128,
        label: &str,
    ) -> Self {
        let response = self
            .app
            .execute_contract(
                owner.clone(),
                self.dca_contract_address.clone(),
                &ExecuteMsg::CreateVault {
                    owner: None,
                    minimum_receive_amount: None,
                    label: Some("label".to_string()),
                    destinations,
                    pool_id: 0,
                    position_type: None,
                    slippage_tolerance: None,
                    swap_amount,
                    time_interval: TimeInterval::Hourly,
                    target_receive_amount: Some(swap_amount),
                    target_start_time_utc_seconds: None,
                    use_dca_plus: None,
                },
                &vec![balance],
            )
            .unwrap();

        self.vault_ids.insert(
            String::from(label),
            Uint128::from_str(
                &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
            )
            .unwrap(),
        );

        // send 2 microns of swap denom out of fin contract to simulate outgoing
        self.app
            .send_tokens(
                self.fin_contract_address.clone(),
                Addr::unchecked(ADMIN),
                &[Coin::new(TWO_MICRONS.into(), DENOM_UOSMO)],
            )
            .unwrap();

        // send 2 micons of receive denom into fin contract to simulate incoming
        self.app
            .send_tokens(
                Addr::unchecked(ADMIN),
                self.fin_contract_address.clone(),
                &[Coin::new(TWO_MICRONS.into(), DENOM_STAKE)],
            )
            .unwrap();

        self
    }

    pub fn with_vault_with_partially_filled_fin_limit_price_trigger(
        mut self,
        owner: &Addr,
        balance: Coin,
        swap_amount: Uint128,
        label: &str,
    ) -> MockApp {
        let response = self
            .app
            .execute_contract(
                owner.clone(),
                self.dca_contract_address.clone(),
                &ExecuteMsg::CreateVault {
                    owner: None,
                    minimum_receive_amount: None,
                    label: Some("label".to_string()),
                    destinations: None,
                    pool_id: 0,
                    position_type: None,
                    slippage_tolerance: None,
                    swap_amount,
                    time_interval: TimeInterval::Hourly,
                    target_receive_amount: Some(swap_amount),
                    target_start_time_utc_seconds: None,
                    use_dca_plus: None,
                },
                &vec![balance],
            )
            .unwrap();

        self.vault_ids.insert(
            String::from(label),
            Uint128::from_str(
                &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
            )
            .unwrap(),
        );

        self.app
            .send_tokens(
                self.fin_contract_address.clone(),
                Addr::unchecked(ADMIN),
                &[Coin {
                    denom: String::from(DENOM_UOSMO),
                    amount: TWO_MICRONS / Uint128::new(2),
                }],
            )
            .unwrap();

        self.app
            .send_tokens(
                Addr::unchecked(ADMIN),
                self.fin_contract_address.clone(),
                &[Coin {
                    denom: String::from(DENOM_STAKE),
                    amount: TWO_MICRONS / Uint128::new(2),
                }],
            )
            .unwrap();

        self
    }

    pub fn with_vault_with_time_trigger(
        mut self,
        owner: &Addr,
        destinations: Option<Vec<Destination>>,
        balance: Coin,
        swap_amount: Uint128,
        label: &str,
        minimum_receive_amount: Option<Uint128>,
        use_dca_plus: Option<bool>,
    ) -> MockApp {
        let response = self
            .app
            .execute_contract(
                owner.clone(),
                self.dca_contract_address.clone(),
                &ExecuteMsg::CreateVault {
                    owner: None,
                    minimum_receive_amount,
                    label: Some("label".to_string()),
                    destinations,
                    pool_id: 0,
                    position_type: None,
                    slippage_tolerance: None,
                    swap_amount,
                    time_interval: use_dca_plus
                        .map_or(TimeInterval::Hourly, |_| TimeInterval::Daily),
                    target_start_time_utc_seconds: Some(Uint64::from(
                        self.app.block_info().time.plus_seconds(2).seconds(),
                    )),
                    target_receive_amount: None,
                    use_dca_plus,
                },
                &vec![balance],
            )
            .unwrap();

        self.vault_ids.insert(
            String::from(label),
            Uint128::from_str(
                &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
            )
            .unwrap(),
        );

        self
    }

    pub fn with_active_vault(
        mut self,
        owner: &Addr,
        destinations: Option<Vec<Destination>>,
        balance: Coin,
        swap_amount: Uint128,
        label: &str,
        minimum_receive_amount: Option<Uint128>,
    ) -> MockApp {
        let response = self
            .app
            .execute_contract(
                owner.clone(),
                self.dca_contract_address.clone(),
                &ExecuteMsg::CreateVault {
                    owner: None,
                    minimum_receive_amount,
                    label: Some("label".to_string()),
                    destinations,
                    pool_id: 0,
                    position_type: None,
                    slippage_tolerance: None,
                    swap_amount,
                    time_interval: TimeInterval::Hourly,
                    target_start_time_utc_seconds: None,
                    target_receive_amount: None,
                    use_dca_plus: None,
                },
                &vec![balance],
            )
            .unwrap();

        self.vault_ids.insert(
            String::from(label),
            Uint128::from_str(
                &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
            )
            .unwrap(),
        );

        self
    }

    pub fn with_inactive_vault(
        mut self,
        owner: &Addr,
        destinations: Option<Vec<Destination>>,
        label: &str,
    ) -> MockApp {
        let response = self
            .app
            .execute_contract(
                owner.clone(),
                self.dca_contract_address.clone(),
                &ExecuteMsg::CreateVault {
                    owner: None,
                    minimum_receive_amount: None,
                    label: Some("label".to_string()),
                    destinations,
                    pool_id: 0,
                    position_type: None,
                    slippage_tolerance: None,
                    swap_amount: Uint128::new(50001),
                    time_interval: TimeInterval::Hourly,
                    target_start_time_utc_seconds: None,
                    target_receive_amount: None,
                    use_dca_plus: None,
                },
                &vec![Coin::new(1, DENOM_UOSMO)],
            )
            .unwrap();

        self.vault_ids.insert(
            String::from(label),
            Uint128::from_str(
                &get_flat_map_for_event_type(&response.events, "wasm").unwrap()["vault_id"],
            )
            .unwrap(),
        );

        self
    }

    pub fn elapse_time(&mut self, seconds: u64) {
        self.app.update_block(|mut block_info| {
            block_info.time = block_info.time.plus_seconds(seconds);
            let seconds_per_block = 5u64;
            block_info.height += seconds / seconds_per_block;
        });
    }

    pub fn get_vault_by_label(&self, label: &str) -> Vault {
        let vault_id = self.vault_ids.get(label).unwrap();
        let vault_response: VaultResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.dca_contract_address.clone(),
                &QueryMsg::GetVault {
                    vault_id: vault_id.to_owned(),
                },
            )
            .unwrap();

        vault_response.vault
    }

    pub fn get_balance(&self, address: &Addr, denom: &str) -> Uint128 {
        self.app
            .wrap()
            .query_balance(address.clone(), denom)
            .unwrap()
            .amount
    }
}

fn _default_swap_handler(info: MessageInfo) -> StdResult<Response> {
    let received_coin = info.funds[0].clone();
    let coin_to_send = match received_coin.denom.as_str() {
        DENOM_UOSMO => Coin {
            denom: String::from(DENOM_STAKE),
            amount: received_coin.amount,
        },
        DENOM_STAKE => Coin {
            denom: String::from(DENOM_UOSMO),
            amount: received_coin.amount,
        },
        _ => panic!("Invalid denom for tests"),
    };

    Ok(Response::new()
        .add_event(
            Event::new("trade")
                .add_attribute("market", "value")
                .add_attribute("base_amount", received_coin.amount.clone())
                .add_attribute("quote_amount", received_coin.amount.clone()),
        )
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin_to_send],
        }))
}

fn _default_submit_order_handler() -> StdResult<Response> {
    Ok(Response::new().add_attribute(
        "order_idx",
        rand::thread_rng().gen_range(0..100).to_string(),
    ))
}

fn _withdraw_filled_order_handler(
    info: MessageInfo,
    order_ids: Option<Vec<Uint128>>,
) -> StdResult<Response> {
    let mut response = Response::new();
    let disbursement_after_maker_fee =
        TWO_MICRONS - TWO_MICRONS * Uint128::new(3) / Uint128::new(4000);
    if let Some(order_ids) = order_ids {
        for _ in order_ids {
            response = response.add_message(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin::new(
                    disbursement_after_maker_fee.into(),
                    DENOM_STAKE.to_string(),
                )],
            })
        }
    }

    Ok(response.add_event(Event::new("transfer").add_attribute(
        "amount",
        format!("{}{}", disbursement_after_maker_fee, DENOM_STAKE),
    )))
}

fn _withdraw_partially_filled_order_handler(
    info: MessageInfo,
    order_ids: Option<Vec<Uint128>>,
) -> StdResult<Response> {
    let mut response = Response::new();
    if let Some(order_ids) = order_ids {
        for _ in order_ids {
            response = response.add_message(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: String::from(DENOM_STAKE),
                    amount: TWO_MICRONS / Uint128::new(2),
                }],
            })
        }
    }
    Ok(response)
}

fn _default_retract_order_handler(info: MessageInfo) -> StdResult<Response> {
    let disbursement_after_maker_fee =
        TWO_MICRONS - TWO_MICRONS * Uint128::new(3) / Uint128::new(4000);
    Ok(Response::new()
        .add_attribute("amount", disbursement_after_maker_fee.to_string())
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: String::from(DENOM_UOSMO),
                amount: disbursement_after_maker_fee,
            }],
        }))
}

fn _retract_partially_filled_order_handler(info: MessageInfo) -> StdResult<Response> {
    Ok(Response::new()
        .add_attribute("amount", (TWO_MICRONS / Uint128::new(2)).to_string())
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: String::from(DENOM_UOSMO),
                amount: TWO_MICRONS / Uint128::new(2),
            }],
        }))
}

fn _default_book_response_handler() -> StdResult<Binary> {
    _book_response_handler(
        String::from(DENOM_STAKE),
        String::from(DENOM_UOSMO),
        Decimal256::from_str("1")?,
        Decimal256::from_str("1")?,
    )
}

fn _book_response_handler(
    _quote_denom: String,
    _base_denom: String,
    _base_price: Decimal256,
    _quote_price: Decimal256,
) -> StdResult<Binary> {
    to_binary(&1)
}

fn _unfilled_order_response(_env: Env) -> StdResult<Binary> {
    to_binary(&1)
}

fn _filled_order_response(_env: Env) -> StdResult<Binary> {
    to_binary(&1)
}

fn _partially_filled_order_response(_env: Env) -> StdResult<Binary> {
    to_binary(&1)
}

fn _default_query_response() -> StdResult<Binary> {
    to_binary(&1)
}

pub fn fin_contract_unfilled_limit_order() -> Box<dyn Contract<Empty>> {
    unimplemented!()
}

pub fn fin_contract_partially_filled_order() -> Box<dyn Contract<Empty>> {
    unimplemented!()
}

pub fn fin_contract_filled_limit_order() -> Box<dyn Contract<Empty>> {
    unimplemented!()
}

pub fn fin_contract_pass_slippage_tolerance() -> Box<dyn Contract<Empty>> {
    unimplemented!()
}

pub fn fin_contract_fail_slippage_tolerance() -> Box<dyn Contract<Empty>> {
    unimplemented!()
}

pub fn fin_contract_high_swap_price() -> Box<dyn Contract<Empty>> {
    unimplemented!()
}

pub fn fin_contract_low_swap_price() -> Box<dyn Contract<Empty>> {
    unimplemented!()
}
