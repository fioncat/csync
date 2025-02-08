use anyhow::{bail, Result};
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

const SECOND: u64 = 1;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;

/// Formats a timestamp into a human-readable duration string.
///
/// # Arguments
/// * `time` - Unix timestamp in seconds
///
/// # Returns
/// A string representing how long ago the timestamp was, e.g. "2 hours ago", "last week", etc.
/// Returns "never" if timestamp is 0, and "now" if less than 30 seconds ago.
///
/// # Examples
/// ```
/// use chrono::Local;
/// use csync_misc::time::format_since;
///
/// let now = Local::now().timestamp() as u64;
/// assert_eq!(format_since(now), "now");
/// assert_eq!(format_since(0), "never");
/// ```
pub fn format_since(time: u64) -> String {
    if time == 0 {
        return String::from("never");
    }
    let now = Local::now().timestamp() as u64;
    let duration = now.saturating_sub(time);

    let unit: &str;
    let value: u64;
    if duration < MINUTE {
        unit = "second";
        if duration < 30 {
            return String::from("now");
        }
        value = duration;
    } else if duration < HOUR {
        unit = "minute";
        value = duration / MINUTE;
    } else if duration < DAY {
        unit = "hour";
        value = duration / HOUR;
    } else if duration < WEEK {
        unit = "day";
        value = duration / DAY;
    } else if duration < MONTH {
        unit = "week";
        value = duration / WEEK;
    } else if duration < YEAR {
        unit = "month";
        value = duration / MONTH;
    } else {
        unit = "year";
        value = duration / YEAR;
    }

    if value > 1 {
        format!("{value} {unit}s ago")
    } else {
        format!("last {unit}")
    }
}

/// Parses a time string into a Unix timestamp.
///
/// # Arguments
/// * `s` - Time string in one of the following formats:
///   - Unix timestamp (e.g. "1234567890")
///   - Date (e.g. "2024-03-20")
///   - Time (e.g. "15:30:00")
///   - DateTime (e.g. "2024-03-20 15:30:00")
///
/// # Returns
/// Unix timestamp in seconds
///
/// # Errors
/// Returns error if the input string cannot be parsed in any of the supported formats
pub fn parse_time(s: &str) -> Result<u64> {
    // First try to parse as u64 timestamp
    if let Ok(timestamp) = s.parse::<u64>() {
        return Ok(timestamp);
    }

    let datetime = if let Ok(time) = NaiveTime::parse_from_str(s, "%H:%M:%S") {
        Local::now().naive_local().date().and_time(time)
    } else if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        date.and_hms_opt(0, 0, 0).unwrap()
    } else if let Ok(datetime) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        datetime
    } else {
        bail!("invalid time '{s}', expected formats: unix timestamp, YYYY-MM-DD, HH:MM:SS, or YYYY-MM-DD HH:MM:SS");
    };

    let local = match Local.from_local_datetime(&datetime).single() {
        Some(local) => local,
        None => bail!("invalid local time"),
    };
    Ok(local.timestamp() as u64)
}

/// Returns a timestamp for the specified number of hours before current time.
///
/// # Arguments
/// * `hours` - Number of hours to subtract from current time
///
/// # Returns
/// Unix timestamp in seconds
pub fn get_time_before_hours(hours: u64) -> u64 {
    let now = Local::now();
    let before = now - Duration::hours(hours as i64);
    before.timestamp() as u64
}
