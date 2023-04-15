use std::fmt::Display;

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

impl Into<String> for LockableDuration {
    fn into(self) -> String {
        String::from(match self {
            LockableDuration::OneDay => "1 day",
            LockableDuration::OneWeek => "1 weel",
            LockableDuration::TwoWeeks => "2 weeks",
        })
    }
}

impl Display for LockableDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
