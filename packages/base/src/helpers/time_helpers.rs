use crate::triggers::trigger::TimeInterval;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use cosmwasm_std::Timestamp;
use std::convert::TryInto;

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

    let mut next_execution_time = get_next_time(last_execution_time, &interval);

    match interval {
        TimeInterval::Monthly => {
            while next_execution_time.le(&current_time) {
                next_execution_time = get_next_time(next_execution_time, &interval);
            }
        }
        _ => {
            let interval_duration_in_seconds =
                get_duration(last_execution_time, &interval).num_seconds();

            let increments_until_future_execution_date = (current_time - last_execution_time)
                .num_seconds()
                .checked_div(interval_duration_in_seconds)
                .expect("should be a valid timestamp")
                + 1;

            next_execution_time = last_execution_time
                + get_duration(last_execution_time, &interval)
                    * increments_until_future_execution_date as i32;
        }
    }

    Timestamp::from_seconds(next_execution_time.timestamp().try_into().unwrap())
}

fn get_duration(previous: DateTime<Utc>, interval: &TimeInterval) -> Duration {
    match interval {
        TimeInterval::HalfHourly => Duration::minutes(30),
        TimeInterval::Hourly => Duration::hours(1),
        TimeInterval::HalfDaily => Duration::hours(12),
        TimeInterval::Daily => Duration::days(1),
        TimeInterval::Weekly => Duration::days(7),
        TimeInterval::Fortnightly => Duration::days(14),
        TimeInterval::Monthly => shift_months(previous, 1) - previous,
    }
}

fn get_next_time(previous: DateTime<Utc>, interval: &TimeInterval) -> DateTime<Utc> {
    previous + get_duration(previous, interval)
}

fn shift_months(date: DateTime<Utc>, months: i32) -> DateTime<Utc> {
    let mut year = date.year() + (date.month() as i32 + months) / 12;
    let mut month = (date.month() as i32 + months) % 12;
    let mut day = date.day();

    if month < 1 {
        year -= 1;
        month += 12;
    }

    day = normalise_day(year, month as u32, day);

    // This is slow but guaranteed to succeed (short of integer overflow)
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

    pub fn assert_expected_next_execution_time(
        last_execution_time: DateTime<Utc>,
        current_time: DateTime<Utc>,
        interval: TimeInterval,
        expected_next_execution_time: DateTime<Utc>,
    ) -> () {
        let current_timestamp =
            Timestamp::from_seconds(current_time.timestamp().try_into().unwrap());
        let last_execution_timestamp =
            Timestamp::from_seconds(last_execution_time.timestamp().try_into().unwrap());

        let next_execution_time = get_next_target_time(
            current_timestamp,
            last_execution_timestamp,
            interval.try_into().unwrap(),
        );

        assert_eq!(
            next_execution_time.seconds(),
            expected_next_execution_time.timestamp() as u64
        );
    }

    fn assert_expected_next_execution_times(
        interval: &TimeInterval,
        last_execution_time: DateTime<Utc>,
        scenarios: Vec<(DateTime<Utc>, DateTime<Utc>)>,
    ) -> () {
        for (current_time, expected_next_execution_time) in scenarios {
            assert_expected_next_execution_time(
                last_execution_time.to_owned(),
                current_time.to_owned(),
                interval.to_owned(),
                expected_next_execution_time.to_owned(),
            );
        }
    }

    #[test]
    fn assert_monthly_next_execution_times() {
        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let scenarios = vec![
            (last_execution_time, shift_months(last_execution_time, 1)),
            (
                last_execution_time + Duration::seconds(1),
                shift_months(last_execution_time, 1),
            ),
            (
                shift_months(last_execution_time, 1) - Duration::seconds(1),
                shift_months(last_execution_time, 1),
            ),
            (
                shift_months(last_execution_time, 1),
                shift_months(last_execution_time, 2),
            ),
            (
                shift_months(last_execution_time, 1) + Duration::seconds(1),
                shift_months(last_execution_time, 2),
            ),
            (
                shift_months(last_execution_time, 33),
                shift_months(last_execution_time, 34),
            ),
        ];

        assert_expected_next_execution_times(
            &TimeInterval::Monthly,
            last_execution_time,
            scenarios,
        );
    }

    #[test]
    fn assert_fortnightly_next_execution_times() {
        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let scenarios = vec![
            (
                last_execution_time,
                last_execution_time + Duration::weeks(2),
            ),
            (
                last_execution_time + Duration::seconds(1),
                last_execution_time + Duration::weeks(2),
            ),
            (
                last_execution_time + Duration::weeks(2) - Duration::seconds(1),
                last_execution_time + Duration::weeks(2),
            ),
            (
                last_execution_time + Duration::weeks(2),
                last_execution_time + Duration::weeks(4),
            ),
            (
                last_execution_time + Duration::weeks(2) + Duration::seconds(1),
                last_execution_time + Duration::weeks(4),
            ),
            (
                last_execution_time + Duration::weeks(33),
                last_execution_time + Duration::weeks(34),
            ),
        ];

        assert_expected_next_execution_times(
            &TimeInterval::Fortnightly,
            last_execution_time,
            scenarios,
        );
    }

    #[test]
    fn assert_weekly_next_execution_times() {
        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let scenarios = vec![
            (
                last_execution_time,
                last_execution_time + Duration::weeks(1),
            ),
            (
                last_execution_time + Duration::seconds(1),
                last_execution_time + Duration::weeks(1),
            ),
            (
                last_execution_time + Duration::weeks(1) - Duration::seconds(1),
                last_execution_time + Duration::weeks(1),
            ),
            (
                last_execution_time + Duration::weeks(1),
                last_execution_time + Duration::weeks(2),
            ),
            (
                last_execution_time + Duration::weeks(1) + Duration::seconds(1),
                last_execution_time + Duration::weeks(2),
            ),
            (
                last_execution_time + Duration::weeks(33),
                last_execution_time + Duration::weeks(34),
            ),
        ];

        assert_expected_next_execution_times(&TimeInterval::Weekly, last_execution_time, scenarios);
    }

    #[test]
    fn assert_daily_next_execution_times() {
        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let scenarios = vec![
            (last_execution_time, last_execution_time + Duration::days(1)),
            (
                last_execution_time + Duration::seconds(1),
                last_execution_time + Duration::days(1),
            ),
            (
                last_execution_time + Duration::days(1) - Duration::seconds(1),
                last_execution_time + Duration::days(1),
            ),
            (
                last_execution_time + Duration::days(1),
                last_execution_time + Duration::days(2),
            ),
            (
                last_execution_time + Duration::days(1) + Duration::seconds(1),
                last_execution_time + Duration::days(2),
            ),
            (
                last_execution_time + Duration::days(33),
                last_execution_time + Duration::days(34),
            ),
        ];

        assert_expected_next_execution_times(&TimeInterval::Daily, last_execution_time, scenarios);
    }

    #[test]
    fn assert_half_daily_next_execution_times() {
        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let scenarios = vec![
            (
                last_execution_time,
                last_execution_time + Duration::hours(12),
            ),
            (
                last_execution_time + Duration::seconds(1),
                last_execution_time + Duration::hours(12),
            ),
            (
                last_execution_time + Duration::hours(12) - Duration::seconds(1),
                last_execution_time + Duration::hours(12),
            ),
            (
                last_execution_time + Duration::hours(12),
                last_execution_time + Duration::hours(24),
            ),
            (
                last_execution_time + Duration::hours(12) + Duration::seconds(1),
                last_execution_time + Duration::hours(24),
            ),
            (
                last_execution_time + Duration::hours(33),
                last_execution_time + Duration::hours(36),
            ),
        ];

        assert_expected_next_execution_times(
            &TimeInterval::HalfDaily,
            last_execution_time,
            scenarios,
        );
    }

    #[test]
    fn assert_hourly_next_execution_times() {
        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let scenarios = vec![
            (
                last_execution_time,
                last_execution_time + Duration::hours(1),
            ),
            (
                last_execution_time + Duration::seconds(1),
                last_execution_time + Duration::hours(1),
            ),
            (
                last_execution_time + Duration::hours(1) - Duration::seconds(1),
                last_execution_time + Duration::hours(1),
            ),
            (
                last_execution_time + Duration::hours(1),
                last_execution_time + Duration::hours(2),
            ),
            (
                last_execution_time + Duration::hours(1) + Duration::seconds(1),
                last_execution_time + Duration::hours(2),
            ),
            (
                last_execution_time + Duration::hours(33),
                last_execution_time + Duration::hours(34),
            ),
        ];

        assert_expected_next_execution_times(&TimeInterval::Hourly, last_execution_time, scenarios);
    }

    #[test]
    fn assert_half_hourly_next_execution_times() {
        let last_execution_time = Utc.ymd(2022, 1, 1).and_hms(1, 0, 0);
        let scenarios = vec![
            (
                last_execution_time,
                last_execution_time + Duration::minutes(30),
            ),
            (
                last_execution_time + Duration::seconds(1),
                last_execution_time + Duration::minutes(30),
            ),
            (
                last_execution_time + Duration::minutes(30) - Duration::seconds(1),
                last_execution_time + Duration::minutes(30),
            ),
            (
                last_execution_time + Duration::minutes(30),
                last_execution_time + Duration::minutes(60),
            ),
            (
                last_execution_time + Duration::minutes(30) + Duration::seconds(1),
                last_execution_time + Duration::minutes(60),
            ),
            (
                last_execution_time + Duration::minutes(333),
                last_execution_time + Duration::minutes(360),
            ),
        ];

        assert_expected_next_execution_times(
            &TimeInterval::HalfHourly,
            last_execution_time,
            scenarios,
        );
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

        let result = get_next_time(current_time, &TimeInterval::Monthly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_week_should_get_next_week() {
        let current_time = Utc.ymd(2022, 5, 1).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 5, 8).and_hms(10, 0, 0);

        let result = get_next_time(current_time, &TimeInterval::Weekly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_week_that_spans_multiple_months_should_get_next_week() {
        let current_time = Utc.ymd(2022, 9, 29).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 10, 6).and_hms(10, 0, 0);

        let result = get_next_time(current_time, &TimeInterval::Weekly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_day_should_get_next_day() {
        let current_time = Utc.ymd(2022, 9, 1).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 9, 2).and_hms(10, 0, 0);

        let result = get_next_time(current_time, &TimeInterval::Daily);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_day_that_spans_multiple_months_should_get_next_day() {
        let current_time = Utc.ymd(2022, 9, 30).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 10, 1).and_hms(10, 0, 0);

        let result = get_next_time(current_time, &TimeInterval::Daily);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_hour_should_get_next_hour() {
        let current_time = Utc.ymd(2022, 10, 1).and_hms(10, 0, 0);

        let expected_time = Utc.ymd(2022, 10, 1).and_hms(11, 0, 0);

        let result = get_next_time(current_time, &TimeInterval::Hourly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }

    #[test]
    fn get_next_time_given_hour_that_spans_multiple_days_should_get_next_hour() {
        let current_time = Utc.ymd(2022, 10, 1).and_hms(23, 0, 0);

        let expected_time = Utc.ymd(2022, 10, 2).and_hms(0, 0, 0);

        let result = get_next_time(current_time, &TimeInterval::Hourly);

        assert_eq!(result.timestamp(), expected_time.timestamp());
    }
}
