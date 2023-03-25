use cosmwasm_std::{CheckedMultiplyRatioError, Decimal, Uint128};

pub fn checked_mul(a: Uint128, b: Decimal) -> Result<Uint128, CheckedMultiplyRatioError> {
    a.checked_multiply_ratio(
        b.atomics(),
        Uint128::new(10).checked_pow(b.decimal_places()).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn multiply_by_one_should_succeed() {
        let a = Uint128::new(100);
        let b = Decimal::one();

        let result = checked_mul(a, b);

        assert_eq!(result.unwrap(), a);
    }

    #[test]
    fn multiply_by_zero_should_succeed() {
        let a = Uint128::new(100);
        let b = Decimal::zero();

        let result = checked_mul(a, b);

        assert_eq!(result.unwrap(), Uint128::zero());
    }

    #[test]
    fn multiply_by_half_should_succeed() {
        let a = Uint128::new(100);
        let b = Decimal::from_str("0.5").unwrap();

        let result = checked_mul(a, b);

        assert_eq!(result.unwrap(), Uint128::new(50));
    }

    #[test]
    fn multiply_by_one_and_half_should_succeed() {
        let a = Uint128::new(100);
        let b = Decimal::from_str("1.5").unwrap();

        let result = checked_mul(a, b);

        assert_eq!(result.unwrap(), Uint128::new(150));
    }

    #[test]
    fn multiply_when_int_not_even_should_succeed() {
        let a = Uint128::new(11);
        let b = Decimal::from_str("1.5").unwrap();

        let result = checked_mul(a, b);

        assert_eq!(result.unwrap(), Uint128::new(16));
    }

    #[test]
    fn multiply_when_max_decimal_should_succeed() {
        let a = Uint128::new(10);
        let b = Decimal::MAX;

        let result = checked_mul(a, b);

        assert_eq!(result.unwrap(), Uint128::new(3402823669209384634633));
    }

    #[test]
    fn multiply_when_max_int_should_fail() {
        let a = Uint128::MAX;
        let b = Decimal::from_str("1.5").unwrap();

        let result = checked_mul(a, b);

        assert_eq!(result.unwrap_err(), CheckedMultiplyRatioError::Overflow);
    }

    #[test]
    fn multiply_when_max_decimal_and_large_int_should_fail() {
        let a = Uint128::new(10).checked_pow(38).unwrap();
        let b = Decimal::MAX;

        let result = checked_mul(a, b);

        assert_eq!(result.unwrap_err(), CheckedMultiplyRatioError::Overflow);
    }

    #[test]
    fn multiply_when_small_decimal_and_large_int_should_succeed() {
        let a = Uint128::new(10).checked_pow(38).unwrap();
        let b = Decimal::from_str("0.015").unwrap();

        let result = checked_mul(a, b);

        assert_eq!(
            result.unwrap(),
            Uint128::new(1500000000000000000000000000000000000)
        );
    }
}
