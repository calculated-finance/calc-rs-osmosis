use crate::msg::InstantiateMsg;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Addr, Coin, Empty, Uint128};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const ALLOWED_Z_CALLER: &str = "allowedzcaller";
pub const DENOM_UKUJI: &str = "ukuji";
pub const DENOM_UTEST: &str = "utest";

pub struct MockApp {
    pub app: App,
    pub staking_router_contract_address: Addr,
}

impl MockApp {
    pub fn new() -> Self {
        let mut app = AppBuilder::new().build(|_, _, _| {});

        let staking_router_contract_address = Self::instantiate_contract(
            &mut app,
            Box::new(ContractWrapper::new(
                crate::contract::execute,
                crate::contract::instantiate,
                crate::contract::query,
            )),
            Addr::unchecked(ADMIN),
            &InstantiateMsg {
                admin: Addr::unchecked(ADMIN),
                allowed_z_callers: vec![Addr::unchecked(ALLOWED_Z_CALLER)],
            },
            "staking-router",
        );

        Self {
            app,
            staking_router_contract_address,
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
                &[Coin::new(amount.u128(), denom.to_string())],
            )
            .unwrap();

        self
    }

    pub fn elapse_time(&mut self, seconds: u64) {
        self.app.update_block(|mut block_info| {
            block_info.time = block_info.time.plus_seconds(seconds);
            let seconds_per_block = 5u64;
            block_info.height += seconds / seconds_per_block;
        });
    }

    pub fn get_balance(&self, address: &Addr, denom: &str) -> Uint128 {
        self.app
            .wrap()
            .query_balance(address.clone(), denom)
            .unwrap()
            .amount
    }
}
