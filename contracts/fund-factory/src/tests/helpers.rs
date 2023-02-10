use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo};

use crate::{contract::instantiate, msg::InstantiateMsg};

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";

pub fn instantiate_contract(deps: DepsMut, env: Env, info: MessageInfo) -> () {
    let msg = InstantiateMsg {
        admin: Addr::unchecked(ADMIN),
        fund_router_code_id: 1,
        fund_core_code_id: 0,
    };

    instantiate(deps, env, info, msg).unwrap();
}
