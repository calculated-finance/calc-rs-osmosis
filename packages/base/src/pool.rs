use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct Pool {
    pub pool_id: u64,
    pub base_denom: String,
    pub quote_denom: String,
}
