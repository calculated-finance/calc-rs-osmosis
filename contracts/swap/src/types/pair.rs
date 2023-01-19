use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum Pair {
    Fin { address: String },
}
