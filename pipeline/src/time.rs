use crate::{store::error, Result};
use chrono::{DateTime, Utc};
use chrono_tz::America::New_York;

#[derive(Clone, Debug)]
pub struct Window {
    pub date: String,
    pub previous: i64,
    pub cutoff: i64,
    pub deadline: i64,
    pub verification: bool,
}

pub fn now() -> i64 {
    Utc::now().timestamp()
}

pub fn at(timestamp: i64) -> Result<DateTime<Utc>> {
    DateTime::from_timestamp(timestamp, 0).ok_or_else(|| error("Invalid timestamp"))
}

pub fn daily(cutoff: i64) -> Result<Window> {
    Ok(Window {
        date: at(cutoff)?
            .with_timezone(&New_York)
            .date_naive()
            .to_string(),
        previous: cutoff
            .checked_sub(86400)
            .ok_or_else(|| error("Date out of range"))?,
        cutoff,
        deadline: cutoff
            .checked_add(3600)
            .ok_or_else(|| error("Date out of range"))?,
        verification: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daily_windows_follow_new_york_dates_and_have_no_fixed_start_hour() {
        for stamp in ["2026-03-08T11:45:00Z", "2026-11-01T12:10:00Z"] {
            let timestamp = DateTime::parse_from_rfc3339(stamp).unwrap().timestamp();
            let window = daily(timestamp).unwrap();
            assert_eq!(window.cutoff - window.previous, 86400);
            assert_eq!(window.deadline - window.cutoff, 3600);
            assert_eq!(window.date, &stamp[..10]);
        }
        let timestamp = DateTime::parse_from_rfc3339("2026-10-25T01:00:00Z")
            .unwrap()
            .timestamp();
        assert_eq!(daily(timestamp).unwrap().date, "2026-10-24");
    }
}
