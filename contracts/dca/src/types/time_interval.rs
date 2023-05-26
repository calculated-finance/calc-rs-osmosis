use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum TimeInterval {
    EveryBlock,
    EveryMinute,
    HalfHourly,
    Hourly,
    HalfDaily,
    Daily,
    Weekly,
    Fortnightly,
    Monthly,
    Custom { seconds: u64 },
}

impl From<TimeInterval> for String {
    fn from(time_interval: TimeInterval) -> Self {
        match time_interval {
            TimeInterval::EveryBlock => "EveryBlock".to_string(),
            TimeInterval::EveryMinute => "EveryMinute".to_string(),
            TimeInterval::HalfHourly => "HalfHourly".to_string(),
            TimeInterval::Hourly => "Hourly".to_string(),
            TimeInterval::HalfDaily => "HalfDaily".to_string(),
            TimeInterval::Daily => "Daily".to_string(),
            TimeInterval::Weekly => "Weekly".to_string(),
            TimeInterval::Fortnightly => "Fortnightly".to_string(),
            TimeInterval::Monthly => "Monthly".to_string(),
            TimeInterval::Custom { seconds } => format!("Custom:{}", seconds),
        }
    }
}
