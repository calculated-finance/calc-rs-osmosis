use cosmwasm_std::{Decimal, Uint128};

pub const TWO_MICRONS: Uint128 = Uint128::new(2);
pub const ONE: Uint128 = Uint128::new(1000000);
pub const TEN: Uint128 = Uint128::new(10000000);
pub const ONE_HUNDRED: Uint128 = Uint128::new(100000000);
pub const ONE_THOUSAND: Uint128 = Uint128::new(1000000000);

pub const ONE_DECIMAL: Decimal = Decimal::new(Uint128::new(1000000000000000000));
