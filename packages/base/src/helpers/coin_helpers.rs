use cosmwasm_std::{Coin, StdResult, Uint128};

pub fn add_to_coin(coin: Coin, amount: Uint128) -> StdResult<Coin> {
    Ok(Coin {
        denom: coin.denom,
        amount: coin.amount.checked_add(amount)?,
    })
}
