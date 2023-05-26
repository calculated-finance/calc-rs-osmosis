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

pub fn subtract_from(coin: Coin, amount: Uint128) -> Coin {
    Coin {
        denom: coin.denom,
        amount: coin.amount.checked_sub(amount).unwrap_or(Uint128::zero()),
    }
}

pub fn empty_of(this: Coin) -> Coin {
    Coin {
        denom: this.denom,
        amount: Uint128::zero(),
    }
}

#[cfg(test)]
mod coin_helpers_tests {
    use crate::helpers::coin::{add, add_to, empty_of, subtract, subtract_from};
    use cosmwasm_std::{Coin, Uint128};

    #[test]
    fn adds_two_coins_with_same_denom() {
        let coin1 = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100),
        };
        let coin2 = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(200),
        };
        let result = add(coin1, coin2).unwrap();
        assert_eq!(result.amount, Uint128::new(300));
        assert_eq!(result.denom, "uusd".to_string());
    }

    #[test]
    fn cannot_add_two_coins_with_different_denoms() {
        let coin1 = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100),
        };
        let coin2 = Coin {
            denom: "ukuj".to_string(),
            amount: Uint128::new(200),
        };
        let result = add(coin1, coin2);
        assert!(result.is_err());
    }

    #[test]
    fn subtracts_two_coins_with_same_denom() {
        let coin1 = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100),
        };
        let coin2 = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(200),
        };
        let result = subtract(&coin1, &coin2).unwrap();
        assert_eq!(result.amount, Uint128::new(0));
        assert_eq!(result.denom, "uusd".to_string());
    }

    #[test]
    fn cannot_subtract_two_coins_with_different_denoms() {
        let coin1 = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100),
        };
        let coin2 = Coin {
            denom: "ukuj".to_string(),
            amount: Uint128::new(200),
        };
        let result = subtract(&coin1, &coin2);
        assert!(result.is_err());
    }

    #[test]
    fn adds_amount_to_coin() {
        let coin = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100),
        };
        let result = add_to(coin, Uint128::new(200));
        assert_eq!(result.amount, Uint128::new(300));
        assert_eq!(result.denom, "uusd".to_string());
    }

    #[test]
    fn subtracts_larger_amount_from_coin() {
        let coin = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100),
        };
        let result = subtract_from(coin, Uint128::new(200));
        assert_eq!(result.amount, Uint128::new(0));
        assert_eq!(result.denom, "uusd".to_string());
    }

    #[test]
    fn subtracts_smaller_amount_from_coin() {
        let coin = Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100),
        };
        let result = subtract_from(coin, Uint128::new(50));
        assert_eq!(result.amount, Uint128::new(50));
        assert_eq!(result.denom, "uusd".to_string());
    }

    #[test]
    fn creates_empty_coin_of_same_denom() {
        let coin = Coin::new(100, "uusd".to_string());
        let empty_coin = empty_of(coin);
        assert_eq!(empty_coin.amount, Uint128::zero());
        assert_eq!(empty_coin.denom, "uusd".to_string());
    }
}
