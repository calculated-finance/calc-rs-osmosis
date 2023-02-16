use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo};

use crate::{contract::instantiate, msg::InstantiateMsg};

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const FUND_ADDRESS: &str = "test";
pub const TOKEN_NAME: &str = "test_token";
pub const ROUTER_ADDRESS: &str = "router";
pub const SWAPPER_ADDRESS: &str = "swapper";
pub const BASE_DENOM: &str = "base_denom";

pub fn instantiate_contract(deps: DepsMut, env: Env, info: MessageInfo) -> () {
    let msg = InstantiateMsg {
        router: Addr::unchecked(info.sender.clone()),
        swapper: Addr::unchecked(SWAPPER_ADDRESS),
        base_denom: TOKEN_NAME.to_string(),
    };

    instantiate(deps, env, info, msg).unwrap();
}
