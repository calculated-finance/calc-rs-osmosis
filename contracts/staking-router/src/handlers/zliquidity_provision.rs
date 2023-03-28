use cosmwasm_std::{Addr, Coin, DepsMut, Env, ReplyOn, Response, SubMsg, Uint128};

use crate::{
    contract::AFTER_LIQUIDITY_PROVISION_REPLY_ID,
    helpers::{create_exec_message, create_protobuf_msg},
    msg::LockupDuration,
    state::{LPCache, LP_CACHE},
    ContractError,
};

use osmosis_helpers::liquidity::create_osmosis_lp_message;

pub fn zliquidity_provision(
    deps: DepsMut,
    env: Env,
    sender_address: Addr,
    pool_id: u64,
    denom: String,
    amount: Uint128,
    duration: LockupDuration,
) -> Result<Response, ContractError> {
    LP_CACHE.save(
        deps.storage,
        &LPCache {
            pool_id,
            sender_address: sender_address.clone(),
            duration,
        },
    )?;

    let lp_msg = create_osmosis_lp_message(
        sender_address.to_string(),
        pool_id,
        Coin::new(amount.into(), denom),
        None,
    );

    let protobuf_msg = create_protobuf_msg(
        "/osmosis.gamm.v1beta1.MsgJoinSwapExternAmountIn".to_string(),
        lp_msg,
    );

    let authz_msg = create_exec_message(env.contract.address, protobuf_msg);

    Ok(Response::new()
        .add_attribute("method", "zliquidity_provision")
        .add_submessage(SubMsg {
            msg: authz_msg,
            gas_limit: None,
            id: AFTER_LIQUIDITY_PROVISION_REPLY_ID,
            reply_on: ReplyOn::Always,
        }))
}
