use cosmwasm_std::{Addr, Coin, Uint128};
use osmosis_std::{
    shim::{Duration},
    types::osmosis::{gamm::v1beta1::MsgJoinSwapExternAmountIn, lockup::MsgLockTokens},
};

pub fn create_osmosis_lp_message(
    sender: String,
    pool_id: u64,
    amount_in: Coin,
    minimum_receive_amount: Option<Uint128>,
) -> MsgJoinSwapExternAmountIn {
    MsgJoinSwapExternAmountIn {
        sender,
        pool_id,
        token_in: Some(amount_in.into()),
        share_out_min_amount: minimum_receive_amount.unwrap_or(Uint128::one()).into(),
    }
}

pub fn create_osmosis_lock_token_message(
    owner: Addr,
    amount: Coin,
    duration_in_seconds: i64,
) -> MsgLockTokens {
    MsgLockTokens {
        owner: owner.to_string(),
        duration: Some(Duration {
            seconds: duration_in_seconds,
            nanos: 0,
        }),
        coins: vec![amount.into()],
    }
}
