use cosmwasm_std::{Coin, Decimal, StdError, StdResult, Uint128};

pub fn add_to_coin(coin: Coin, amount: Uint128) -> Coin {
    Coin {
        denom: coin.denom,
        amount: coin.amount + amount,
    }
}

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

pub fn subtract(this: &Coin, other: &Coin) -> StdResult<Coin> {
    if this.denom != other.denom {
        return Err(StdError::generic_err(format!(
            "Cannot subtract coins of different denominations: {} and {}",
            this.denom, other.denom
        )));
    }

    Ok(Coin {
        denom: this.denom.clone(),
        amount: this
            .amount
            .checked_sub(other.amount)
            .unwrap_or(Uint128::zero()),
    })
}

pub fn multiply(this: Coin, factor: Decimal) -> StdResult<Coin> {
    Ok(Coin {
        denom: this.denom,
        amount: this.amount * factor,
    })
}

pub fn empty_of(this: Coin) -> Coin {
    Coin {
        denom: this.denom,
        amount: Uint128::zero(),
    }
}
