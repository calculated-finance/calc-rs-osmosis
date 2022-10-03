use std::convert::TryInto;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use cosmwasm_std::Timestamp;

use crate::triggers::time_trigger::TimeInterval;

pub fn target_time_elapsed(current_time: Timestamp, target_execution_time: Timestamp) -> bool {
    if current_time.seconds().ge(&target_execution_time.seconds()) {
        return true;
    } else {
        return false;
    }
}

pub fn get_next_target_time(
    current_timestamp: Timestamp,
    last_execution_timestamp: Timestamp,
    interval: TimeInterval,
) -> Timestamp {
    let current_time = Utc.timestamp(current_timestamp.seconds().try_into().unwrap(), 0);
    let last_execution_time =
        Utc.timestamp(last_execution_timestamp.seconds().try_into().unwrap(), 0);

    let next_time = get_next_time(last_execution_time, interval.clone());

    if current_time.ge(&next_time) {
        match interval {
            TimeInterval::Monthly => {
                let next = recover_monthly_interval(current_time, last_execution_time);
                return Timestamp::from_seconds(next.timestamp().try_into().unwrap());
            }
            TimeInterval::Weekly => {
                let next = recover_weekly_interval(current_time, last_execution_time);
                return Timestamp::from_seconds(next.timestamp().try_into().unwrap());
            }
            TimeInterval::Daily => {
                let next = recover_daily_interval(current_time, last_execution_time);
                return Timestamp::from_seconds(next.timestamp().try_into().unwrap());
            }
            TimeInterval::Hourly => {
                let next = recover_hourly_interval(current_time, last_execution_time);
                return Timestamp::from_seconds(next.timestamp().try_into().unwrap());
            }
        }
    } else {
        return Timestamp::from_seconds(next_time.timestamp().try_into().unwrap());
    }
}

fn get_next_time(previous: DateTime<Utc>, interval: TimeInterval) -> DateTime<Utc> {
    match interval {
        TimeInterval::Monthly => shift_months(previous, 1),
        TimeInterval::Weekly => previous + Duration::days(7),
        TimeInterval::Daily => previous + Duration::days(1),
        TimeInterval::Hourly => previous + Duration::hours(1),
    }
}

fn recover_monthly_interval(
    current_time: DateTime<Utc>,
    last_execution_time: DateTime<Utc>,
) -> DateTime<Utc> {
    let mut new_time = last_execution_time.clone();
    while current_time.ge(&new_time) {
        new_time = shift_months(new_time, 1);
    }

    return new_time;
}

fn recover_weekly_interval(
    current_time: DateTime<Utc>,
    last_execution_time: DateTime<Utc>,
) -> DateTime<Utc> {
    let mut new_time = last_execution_time.clone();
    while current_time.ge(&new_time) {
        new_time += Duration::days(7)
    }

    return new_time;
}

fn recover_daily_interval(
    current_time: DateTime<Utc>,
    last_execution_time: DateTime<Utc>,
) -> DateTime<Utc> {
    let mut new_time = last_execution_time.clone();
    while current_time.ge(&new_time) {
        new_time += Duration::days(1)
    }

    return new_time;
}

fn recover_hourly_interval(
    current_time: DateTime<Utc>,
    last_execution_time: DateTime<Utc>,
) -> DateTime<Utc> {
    let mut new_time = last_execution_time.clone();
    while current_time.ge(&new_time) {
        new_time += Duration::hours(1)
    }

    return new_time;
}

fn shift_months<D: Datelike>(date: D, months: i32) -> D {
    let mut year = date.year() + (date.month() as i32 + months) / 12;
    let mut month = (date.month() as i32 + months) % 12;
    let mut day = date.day();

    if month < 1 {
        year -= 1;
        month += 12;
    }

    day = normalise_day(year, month as u32, day);

    // This is slow but guaranteed to succeed (short of interger overflow)
    if day <= 28 {
        date.with_day(day)
            .unwrap()
            .with_month(month as u32)
            .unwrap()
            .with_year(year)
            .unwrap()
    } else {
        date.with_day(1)
            .unwrap()
            .with_month(month as u32)
            .unwrap()
            .with_year(year)
            .unwrap()
            .with_day(day)
            .unwrap()
    }
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn normalise_day(year: i32, month: u32, day: u32) -> u32 {
    if day <= 28 {
        day
    } else if month == 2 {
        28 + is_leap_year(year) as u32
    } else if day == 31 && (month == 4 || month == 6 || month == 9 || month == 11) {
        30
    } else {
        day
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Uint64;

    use super::*;

    // system is ok
    // get next time monthly
    // get execution time that is in future
    #[test]
    fn get_next_execution_time_monthly_should_get_next_month() {
        // current time is 15 days since last execution - so in expected time frame
        let mock_current_time = Utc.ymd(2022, 1, 15).and_hms(1, 0, 1);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 2, 1).and_hms(1, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Monthly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_monthly_from_late_last_execution_should_get_next_month() {
        // current time is 1 month and 5 days since last execution - so need to find next possible time
        let mock_current_time = Utc.ymd(2022, 2, 15).and_hms(1, 0, 0);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 3, 1).and_hms(1, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Monthly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_monthly_from_late_last_execution_should_get_today() {
        // current time is 2 months and 23 hours since last execution - so need to find next possible time
        let mock_current_time = Utc.ymd(2022, 3, 1).and_hms(1, 0, 0);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(2, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 3, 1).and_hms(2, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Monthly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_weekly_should_get_next_week() {
        // current time is 4 days since last execution - so in expected time frame
        let mock_current_time = Utc.ymd(2022, 1, 5).and_hms(1, 0, 1);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 8).and_hms(1, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Weekly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_weekly_from_late_last_execution_should_get_next_week() {
        // current time 8 days since last execution - so need to find next possible time
        let mock_current_time = Utc.ymd(2022, 1, 9).and_hms(1, 0, 0);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 15).and_hms(1, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Weekly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_weekly_from_late_last_execution_should_get_today() {
        // current time is 13 days and 23 hours since last execution - so need to find next possible time
        let mock_current_time = Utc.ymd(2022, 1, 15).and_hms(1, 0, 0);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(2, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 15).and_hms(2, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Weekly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_daily_should_get_next_day() {
        // current time is 12 hours since last execution = so in expected time frame
        let mock_current_time = Utc.ymd(2022, 1, 1).and_hms(12, 0, 1);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 2).and_hms(1, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Daily,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_daily_from_late_last_execution_should_get_next_day() {
        // current time 8 days since last execution - so need to find next possible time
        let mock_current_time = Utc.ymd(2022, 1, 9).and_hms(1, 0, 0);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 10).and_hms(1, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Daily,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_daily_from_late_last_execution_should_get_today() {
        // current time is 13 days and 23 hours since last execution - so need to find next possible time
        let mock_current_time = Utc.ymd(2022, 1, 15).and_hms(1, 0, 0);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(2, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 15).and_hms(2, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Daily,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_hourly_should_get_next_hour() {
        // current time is 30 minutes since last execution - so in expected time frame
        let mock_current_time = Utc.ymd(2022, 1, 1).and_hms(1, 30, 1);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 1).and_hms(2, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Hourly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_hourly_from_late_last_execution_should_get_next_hour() {
        // current time 8 days since last execution - so need to find next possible time
        let mock_current_time = Utc.ymd(2022, 1, 9).and_hms(1, 0, 0);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 9).and_hms(2, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Hourly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn get_next_execution_time_hourly_from_late_last_execution_should_get_today() {
        // current time is 13 days and 23 hours since last execution - so need to find next possible time
        let mock_current_time = Utc.ymd(2022, 1, 15).and_hms(1, 58, 0);
        let mock_current_timestamp =
            Timestamp::from_seconds(mock_current_time.timestamp().try_into().unwrap());

        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(2, 0, 0);
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let expected_next_execution_time = Utc.ymd(2022, 1, 15).and_hms(2, 0, 0);
        let expected_next_execution_timestamp =
            Timestamp::from_seconds(expected_next_execution_time.timestamp().try_into().unwrap());

        let actual_next_execution_time = get_next_target_time(
            mock_current_timestamp,
            last_execution_timestamp,
            TimeInterval::Hourly,
        );

        assert_eq!(
            expected_next_execution_timestamp,
            actual_next_execution_time
        )
    }

    #[test]
    fn execution_interval_elapsed_with_time_in_past_should_return_true() {
        let current_time = Timestamp::from_seconds(Uint64::new(17000000000).try_into().unwrap());
        let time_in_the_past =
            Timestamp::from_seconds(Uint64::new(16000000000).try_into().unwrap());

        let result = target_time_elapsed(current_time, time_in_the_past);

        assert_eq!(result, true);
    }

    #[test]
    fn execution_interval_elapsed_with_time_in_future_should_return_false() {
        let current_time = Timestamp::from_seconds(Uint64::new(17000000000).try_into().unwrap());
        let time_in_the_future =
            Timestamp::from_seconds(Uint64::new(18000000000).try_into().unwrap());

        let result = target_time_elapsed(current_time, time_in_the_future);

        assert_eq!(result, false);
    }

    #[test]
    fn execution_interval_elapsed_with_current_time_should_return_true() {
        let current_time = Timestamp::from_seconds(Uint64::new(17000000000).try_into().unwrap());
        let time_in_the_future =
            Timestamp::from_seconds(Uint64::new(17000000000).try_into().unwrap());

        let result = target_time_elapsed(current_time, time_in_the_future);

        assert_eq!(result, true);
    }

    #[test]
    fn get_next_time_given_month_should_get_next_month() {
        let current_time = Utc.ymd(2022, 5, 1).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 6, 1).and_hms(10, 0, 0);

        let result = get_next_time(current_time, TimeInterval::Monthly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_week_should_get_next_week() {
        let current_time = Utc.ymd(2022, 5, 1).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 5, 8).and_hms(10, 0, 0);

        let result = get_next_time(current_time, TimeInterval::Weekly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_week_that_spans_multiple_months_should_get_next_week() {
        let current_time = Utc.ymd(2022, 9, 29).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 10, 6).and_hms(10, 0, 0);

        let result = get_next_time(current_time, TimeInterval::Weekly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_day_should_get_next_day() {
        let current_time = Utc.ymd(2022, 9, 1).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 9, 2).and_hms(10, 0, 0);

        let result = get_next_time(current_time, TimeInterval::Daily);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_day_that_spans_multiple_months_should_get_next_day() {
        let current_time = Utc.ymd(2022, 9, 30).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 10, 1).and_hms(10, 0, 0);

        let result = get_next_time(current_time, TimeInterval::Daily);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_hour_should_get_next_hour() {
        let current_time = Utc.ymd(2022, 10, 1).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 10, 1).and_hms(11, 0, 0);

        let result = get_next_time(current_time, TimeInterval::Hourly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_hour_that_spans_multiple_days_should_get_next_hour() {
        let current_time = Utc.ymd(2022, 10, 1).and_hms(23, 0, 0);

        let expected_time = Utc.ymd(2022, 10, 2).and_hms(0, 0, 0);

        let result = get_next_time(current_time, TimeInterval::Hourly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }
}
