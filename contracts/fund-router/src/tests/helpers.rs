use cosmwasm_std::{DepsMut, Env, MessageInfo};

use crate::{contract::instantiate, msg::InstantiateMsg};

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const FUND_ADDRESS: &str = "test";
pub const TOKEN_NAME: &str = "test_token";

pub fn instantiate_contract(deps: DepsMut, env: Env, info: MessageInfo) -> () {
    let msg = InstantiateMsg {
        token_name: TOKEN_NAME.to_string(),
    };

    instantiate(deps, env, info, msg).unwrap();
}
