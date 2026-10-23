use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::America::New_York;
use worker::{Error, Result};

#[derive(Clone, Debug)]
pub struct Window {
    pub date: String,
    pub previous: i64,
    pub cutoff: i64,
    pub deadline: i64,
    pub verification: bool,
}

pub fn now() -> i64 {
    (js_sys::Date::now() / 1000.0) as i64
}

pub fn at(timestamp: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| Error::RustError("Invalid timestamp".into()))
}

pub fn for_date(date: NaiveDate) -> Result<Window> {
    let previous = date
        .checked_sub_signed(Duration::days(1))
        .ok_or_else(|| Error::RustError("Date out of range".into()))?;
    let local = |day: NaiveDate, hour, minute| -> Result<i64> {
        let time = day
            .and_hms_opt(hour, minute, 0)
            .ok_or_else(|| Error::RustError("Invalid local time".into()))?;
        New_York
            .from_local_datetime(&time)
            .single()
            .map(|d| d.timestamp())
            .ok_or_else(|| Error::RustError("Ambiguous local time".into()))
    };
    Ok(Window {
        date: date.to_string(),
        previous: local(previous, 6, 30)?,
        cutoff: local(date, 6, 30)?,
        deadline: local(date, 7, 30)?,
        verification: false,
    })
}

pub fn today(timestamp: i64) -> Result<Window> {
    for_date(at(timestamp)?.with_timezone(&New_York).date_naive())
}

pub fn expected(timestamp: i64) -> Result<Window> {
    let current = today(timestamp)?;
    if timestamp < current.deadline {
        let date = at(timestamp)?.with_timezone(&New_York).date_naive() - Duration::days(1);
        for_date(date)
    } else {
        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn day(value: &str) -> Window {
        for_date(NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()).unwrap()
    }
    #[test]
    fn actual_new_york_dst_boundaries() {
        assert_eq!(
            day("2026-03-08").cutoff - day("2026-03-08").previous,
            23 * 3600
        );
        assert_eq!(
            day("2026-11-01").cutoff - day("2026-11-01").previous,
            25 * 3600
        );
        for date in ["2026-03-08", "2026-11-01", "2026-10-24", "2027-01-01"] {
            let window = day(date);
            assert_eq!(window.deadline - window.cutoff, 3600);
            assert_eq!(expected(window.deadline).unwrap().date, date);
            assert_ne!(expected(window.deadline - 1).unwrap().date, date);
        }
        assert_eq!(
            at(day("2026-01-01").cutoff).unwrap().to_rfc3339(),
            "2026-01-01T11:30:00+00:00"
        );
        assert_eq!(
            at(day("2026-07-01").cutoff).unwrap().to_rfc3339(),
            "2026-07-01T10:30:00+00:00"
        );
    }
}
