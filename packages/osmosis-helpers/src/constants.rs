use cosmwasm_std::{Decimal, Uint128};

pub const ONE_MICRON: Uint128 = Uint128::new(1);
pub const TWO_MICRONS: Uint128 = Uint128::new(2);
pub const FIVE_MICRONS: Uint128 = Uint128::new(5);
pub const TEN_MICRONS: Uint128 = Uint128::new(10);
pub const ONE: Uint128 = Uint128::new(1000000);
pub const TEN: Uint128 = Uint128::new(10000000);
pub const ONE_HUNDRED: Uint128 = Uint128::new(100000000);
pub const ONE_THOUSAND: Uint128 = Uint128::new(1000000000);

pub const ONE_DECIMAL: Decimal = Decimal::new(Uint128::new(1000000000000000000));

pub const OSMOSIS_SWAP_FEE_RATE: &str = "0.001";
