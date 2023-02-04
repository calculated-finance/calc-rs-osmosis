use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};

#[cw_serde]
pub struct Callback {
    pub address: Addr,
    pub msg: Binary,
}
