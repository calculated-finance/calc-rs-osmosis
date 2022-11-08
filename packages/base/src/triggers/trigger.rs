use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal256, Timestamp, Uint128};
use enum_as_inner::EnumAsInner;

#[cw_serde]
pub enum TimeInterval {
    HalfHourly,
    Hourly,
    HalfDaily,
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
}

#[derive(EnumAsInner)]
#[cw_serde]
pub enum TriggerConfiguration {
    Time {
        target_time: Timestamp,
    },
    FinLimitOrder {
        target_price: Decimal256,
        order_idx: Option<Uint128>,
    },
}

#[cw_serde]
pub struct Trigger {
    pub vault_id: Uint128,
    pub configuration: TriggerConfiguration,
}
