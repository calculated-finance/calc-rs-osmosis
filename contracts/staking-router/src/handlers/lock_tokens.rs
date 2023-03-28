use cosmwasm_std::{DepsMut, Env, Reply, Response};

use crate::{
    helpers::{create_exec_message, create_protobuf_msg},
    msg::LockupDuration,
    state::LP_CACHE,
    ContractError,
};

use osmosis_helpers::liquidity::create_osmosis_lock_token_message;

pub fn lock_tokens(deps: DepsMut, env: Env, _: Reply) -> Result<Response, ContractError> {
    let lp_cache = LP_CACHE.load(deps.storage)?;

    let duration_in_seconds = match lp_cache.duration {
        LockupDuration::OneDay => 86400,
        LockupDuration::OneWeek => 604800,
        LockupDuration::TwoWeeks => 1209600,
    };

    let balance = deps.querier.query_balance(
        lp_cache.sender_address.clone(),
        format!("gamm/pool/{}", lp_cache.pool_id),
    )?;

    let lock_token_msg =
        create_osmosis_lock_token_message(lp_cache.sender_address, balance, duration_in_seconds);

    let protobuf_msg =
        create_protobuf_msg("/osmosis.lockup.MsgLockTokens".to_string(), lock_token_msg);

    let authz_msg = create_exec_message(env.contract.address, protobuf_msg);

    Ok(Response::new()
        .add_attribute("method", "lock_tokens")
        .add_message(authz_msg))
}
