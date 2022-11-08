use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

#[cw_serde]
pub struct PriceDeltaLimit {
    pub duration_in_seconds: u64,
    pub max_price_delta: Decimal,
}
