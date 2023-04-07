use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Timestamp, Uint128};

#[cw_serde]
pub enum TriggerConfiguration {
    Time { target_time: Timestamp },
}

#[cw_serde]
pub struct Trigger {
    pub vault_id: Uint128,
    pub configuration: TriggerConfiguration,
}
