use cosmwasm_std::{Addr, Coin, Env, QuerierWrapper, ReplyOn, StdResult, SubMsg, Uint128};
use osmosis_std::{
    shim::Duration,
    types::osmosis::{gamm::v1beta1::MsgJoinSwapExternAmountIn, lockup::MsgLockTokens},
};

pub fn create_osmosis_liquidity_provision_message(
    sender: String,
    pool_id: u64,
    amount_in: Coin,
    minimum_receive_amount: Option<Uint128>,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let join_liquidity_pool = MsgJoinSwapExternAmountIn {
        sender,
        pool_id,
        token_in: Some(amount_in.into()),
        share_out_min_amount: minimum_receive_amount.unwrap_or(Uint128::one()).into(),
    };

    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: join_liquidity_pool.into(),
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}

pub fn create_osmosis_lock_token_message(
    querier: QuerierWrapper,
    env: Env,
    pool_id: u64,
    owner: Addr,
    reply_id: Option<u64>,
    reply_on: Option<ReplyOn>,
) -> StdResult<SubMsg> {
    let balance = querier.query_balance(
        env.contract.address.clone(),
        format!("gamm/pool/{}", pool_id),
    )?;

    let seconds_in_24_hours = 86400;

    let lock = MsgLockTokens {
        owner: owner.to_string(),
        duration: Some(Duration {
            seconds: seconds_in_24_hours,
            nanos: 0,
        }),
        coins: vec![balance.into()],
    };
    Ok(SubMsg {
        id: reply_id.unwrap_or(0),
        msg: lock.into(),
        gas_limit: None,
        reply_on: reply_on.unwrap_or(ReplyOn::Never),
    })
}
