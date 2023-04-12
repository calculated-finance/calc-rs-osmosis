use cosmwasm_schema::cw_serde;
use osmosis_std::shim::Duration;

#[cw_serde]
pub enum PostExecutionAction {
    Send,
    ZDelegate,
    ZProvideLiquidity {
        pool_id: u64,
        duration: LockableDuration,
    },
}

#[cw_serde]
pub enum LockableDuration {
    OneDay,
    OneWeek,
    TwoWeeks,
}

impl From<LockableDuration> for Duration {
    fn from(ld: LockableDuration) -> Self {
        Duration {
            seconds: match ld {
                LockableDuration::OneDay => 60 * 60 * 24,
                LockableDuration::OneWeek => 60 * 60 * 24 * 7,
                LockableDuration::TwoWeeks => 60 * 60 * 24 * 14,
            },
            nanos: 0,
        }
    }
}
