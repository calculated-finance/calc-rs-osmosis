use cosmwasm_std::{Coin, StdError, StdResult, Uint128};

pub fn add(this: Coin, other: Coin) -> StdResult<Coin> {
    if this.denom != other.denom {
        return Err(StdError::generic_err(format!(
            "Cannot add coins of different denominations: {} and {}",
            this.denom, other.denom
        )));
    }

    Ok(Coin {
        denom: this.denom,
        amount: this.amount.checked_add(other.amount)?,
    })
}

pub fn subtract(from: &Coin, other: &Coin) -> StdResult<Coin> {
    if from.denom != other.denom {
        return Err(StdError::generic_err(format!(
            "Cannot subtract coins of different denominations: {} and {}",
            from.denom, other.denom
        )));
    }

    Ok(Coin {
        denom: from.denom.clone(),
        amount: from
            .amount
            .checked_sub(other.amount)
            .unwrap_or(Uint128::zero()),
    })
}

pub fn add_to(coin: Coin, amount: Uint128) -> Coin {
    Coin {
        denom: coin.denom,
        amount: coin.amount + amount,
    }
}

pub fn empty_of(this: Coin) -> Coin {
    Coin {
        denom: this.denom,
        amount: Uint128::zero(),
    }
}
