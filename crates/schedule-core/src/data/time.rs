/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Date/time format constants and parsing helpers used throughout the codebase.

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize, de::Deserializer, ser::Serializer};

/// Internal ISO-8601 storage format for panel start/end times (no timezone).
pub const STORAGE_FMT: &str = "%Y-%m-%dT%H:%M:%S";

/// UTC timestamp format used in JSON metadata fields (generated, modified, etc.).
pub const STORAGE_TS_FMT: &str = "%Y-%m-%dT%H:%M:%SZ";

/// Human-readable display format written into XLSX cell values.
pub const XLSX_DISPLAY_FMT: &str = "%-m/%-d/%Y %-I:%M %p";

/// Timestamp format used for the Grid sheet and Timestamp sheet cells.
pub const LOCAL_TS_FMT: &str = "%Y-%m-%d %H:%M:%S";

/// Parse an internal storage string (`STORAGE_FMT`) into a `NaiveDateTime`.
/// Returns `None` if the string does not match the storage format.
pub fn parse_storage(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s.trim(), STORAGE_FMT).ok()
}

/// Format a `NaiveDateTime` as the internal storage string (`STORAGE_FMT`).
pub fn format_storage(dt: NaiveDateTime) -> String {
    dt.format(STORAGE_FMT).to_string()
}

/// Format a UTC `DateTime` as an ISO-8601 UTC timestamp (`STORAGE_TS_FMT`).
pub fn format_storage_ts(dt: DateTime<Utc>) -> String {
    dt.format(STORAGE_TS_FMT).to_string()
}

/// Format a `NaiveDateTime` as an XLSX cell display string (`XLSX_DISPLAY_FMT`).
pub fn format_display(dt: NaiveDateTime) -> String {
    dt.format(XLSX_DISPLAY_FMT).to_string()
}

/// Serialize an Option<NaiveDateTime> for JSON
pub fn serialize_optional_datetime<S>(
    datetime: &Option<NaiveDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match datetime {
        Some(dt) => {
            let formatted = format_storage(*dt);
            Some(formatted).serialize(serializer)
        }
        None => Option::<String>::None.serialize(serializer),
    }
}

/// Deserialize an Option<NaiveDateTime> from JSON
pub fn deserialize_optional_datetime<'de, D>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let option_str: Option<String> = Option::deserialize(deserializer)?;
    match option_str {
        Some(s) => {
            let parsed = parse_storage(&s).ok_or_else(|| {
                serde::de::Error::custom(format!("Invalid datetime format: {}", s))
            })?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

/// Serialize an Option<chrono::Duration> for JSON (as minutes)
pub fn serialize_optional_duration<S>(
    duration: &Option<Duration>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match duration {
        Some(d) => {
            let minutes = d.num_minutes();
            Some(minutes).serialize(serializer)
        }
        None => Option::<i64>::None.serialize(serializer),
    }
}

/// Deserialize an Option<chrono::Duration> from JSON (as minutes)
pub fn deserialize_optional_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let option_minutes: Option<i64> = Option::deserialize(deserializer)?;
    match option_minutes {
        Some(minutes) => {
            if minutes >= 0 {
                Ok(Some(Duration::minutes(minutes)))
            } else {
                Err(serde::de::Error::custom("Duration cannot be negative"))
            }
        }
        None => Ok(None),
    }
}

/// Parse a datetime string in any of the formats found in schedule spreadsheets.
///
/// Accepted formats (in order of preference):
/// - ISO 8601: `2026-06-26T14:00:00`
/// - ISO with space: `2026-06-26 14:00:00`
/// - US M/D/YY H:MM: `6/26/26 14:00`
/// - US M/D/YYYY H:MM [AM/PM]: `6/26/2026 2:00 PM`
/// - Excel serial number (float string)
pub fn parse_datetime(text: &str) -> Option<NaiveDateTime> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // ISO format (with T separator)
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, STORAGE_FMT) {
        return Some(dt);
    }
    // ISO with space separator
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }

    // M-DD-YY HH:MM (e.g., "6-27-26 18:00")
    let re_short = Regex::new(r"^(\d{1,2})-(\d{1,2})-(\d{2})\s+(\d{1,2}):(\d{2})$").ok()?;
    if let Some(caps) = re_short.captures(text) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let year_short: u32 = caps[3].parse().ok()?;
        let hour: u32 = caps[4].parse().ok()?;
        let minute: u32 = caps[5].parse().ok()?;

        let year = if year_short >= 70 {
            1900 + year_short as i32
        } else {
            2000 + year_short as i32
        };

        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
        let time = chrono::NaiveTime::from_hms_opt(hour, minute, 0)?;
        return Some(NaiveDateTime::new(date, time));
    }

    // M/DD/YYYY H:MM [AM/PM]
    let re_us =
        Regex::new(r"^(\d{1,2})/(\d{1,2})/(\d{4})\s+(\d{1,2}):(\d{2})(?::(\d{2}))?\s*(AM|PM)?$")
            .ok()?;
    if let Some(caps) = re_us.captures(text) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let year: i32 = caps[3].parse().ok()?;
        let mut hour: u32 = caps[4].parse().ok()?;
        let minute: u32 = caps[5].parse().ok()?;
        let second: u32 = caps
            .get(6)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);

        if let Some(ampm) = caps.get(7) {
            match ampm.as_str() {
                "PM" if hour < 12 => hour += 12,
                "AM" if hour == 12 => hour = 0,
                _ => {}
            }
        }

        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
        let time = chrono::NaiveTime::from_hms_opt(hour, minute, second)?;
        return Some(NaiveDateTime::new(date, time));
    }

    None
}

/// Parse a duration string into minutes.
///
/// Accepted formats:
/// - `H:MM` or `HH:MM` — hours and minutes
/// - Plain integer — minutes
pub fn parse_duration_str(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let re_hm = Regex::new(r"^(\d+):(\d{1,2})$").ok()?;
    if let Some(caps) = re_hm.captures(text) {
        let hours: u32 = caps[1].parse().ok()?;
        let minutes: u32 = caps[2].parse().ok()?;
        return Some(hours * 60 + minutes);
    }

    if let Ok(minutes) = text.parse::<u32>() {
        return Some(minutes);
    }

    None
}

// Serialization modules for PanelTiming enum variants
pub mod datetime_option {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(datetime: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = format_storage(*datetime);
        serializer.serialize_str(&formatted)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_datetime(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("Invalid datetime format: {}", s)))
    }
}

pub mod duration_option {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let minutes = duration.num_minutes();
        serializer.serialize_i64(minutes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let minutes = i64::deserialize(deserializer)?;
        Ok(Duration::minutes(minutes))
    }
}

/// Represents different ways a time range can be specified
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeRange {
    /// No timing information specified
    Unspecified,
    /// Unscheduled but has duration specified
    #[serde(with = "duration_option")]
    UnspecifiedWithDuration(Duration),
    /// Unscheduled but has start time specified (no duration yet)
    #[serde(with = "datetime_option")]
    UnspecifiedWithStart(NaiveDateTime),
    /// Fully scheduled with start time + duration
    Scheduled {
        #[serde(with = "datetime_option")]
        start_time: NaiveDateTime,
        #[serde(with = "duration_option")]
        duration: Duration,
    },
}

impl Default for TimeRange {
    fn default() -> Self {
        TimeRange::Unspecified
    }
}

impl TimeRange {
    /// Create a new Scheduled time range with validation
    pub fn new_scheduled(start_time: NaiveDateTime, duration: Duration) -> Result<Self, String> {
        if duration < Duration::zero() {
            return Err("Duration cannot be negative".to_string());
        }
        Ok(TimeRange::Scheduled {
            start_time,
            duration,
        })
    }

    /// Create a time range from start and end times
    pub fn from_start_end(
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    ) -> Result<Self, String> {
        let duration = end_time - start_time;
        if duration < Duration::zero() {
            return Err("End time must be after start time".to_string());
        }
        Ok(TimeRange::Scheduled {
            start_time,
            duration,
        })
    }

    /// Returns the start time if available
    pub fn start_time(&self) -> Option<NaiveDateTime> {
        match self {
            TimeRange::Unspecified => None,
            TimeRange::UnspecifiedWithDuration(_) => None,
            TimeRange::UnspecifiedWithStart(start_time) => Some(*start_time),
            TimeRange::Scheduled { start_time, .. } => Some(*start_time),
        }
    }

    /// Returns the duration if available
    pub fn duration(&self) -> Option<Duration> {
        match self {
            TimeRange::Unspecified => None,
            TimeRange::UnspecifiedWithDuration(duration) => Some(*duration),
            TimeRange::UnspecifiedWithStart(_) => None,
            TimeRange::Scheduled { duration, .. } => Some(*duration),
        }
    }

    /// Returns the effective end time (start + duration) if available
    pub fn effective_end_time(&self) -> Option<NaiveDateTime> {
        match self {
            TimeRange::Unspecified => None,
            TimeRange::UnspecifiedWithDuration(_) => None,
            TimeRange::UnspecifiedWithStart(_) => None,
            TimeRange::Scheduled {
                start_time,
                duration,
            } => Some(*start_time + *duration),
        }
    }

    /// Check if this time range overlaps with another time range
    pub fn overlaps_with(&self, other: &TimeRange) -> bool {
        if let (Some(start_a), Some(end_a), Some(start_b), Some(end_b)) = (
            self.start_time(),
            self.effective_end_time(),
            other.start_time(),
            other.effective_end_time(),
        ) {
            // Overlap if A starts before B ends AND B starts before A ends
            start_a < end_b && start_b < end_a
        } else {
            false
        }
    }

    /// Check if this time range contains a specific datetime
    pub fn contains_datetime(&self, datetime: NaiveDateTime) -> bool {
        if let (Some(start_time), Some(end_time)) = (self.start_time(), self.effective_end_time()) {
            // Contains if start <= datetime < end
            start_time <= datetime && datetime < end_time
        } else {
            false
        }
    }

    /// Returns the start time as a formatted string, or None if not set
    pub fn start_time_str(&self) -> Option<String> {
        self.start_time().map(|dt| format_storage(dt))
    }

    /// Returns the end time as a formatted string, or None if not set
    pub fn end_time_str(&self) -> Option<String> {
        self.effective_end_time().map(|dt| format_storage(dt))
    }

    /// Returns the duration in minutes as a string, or None if not set
    pub fn duration_minutes_str(&self) -> Option<String> {
        self.duration().map(|d| d.num_minutes().to_string())
    }

    /// Sets the start time from a string using the storage format
    /// Returns true if parsing succeeded, false otherwise
    pub fn set_start_time_from_str(&mut self, time_str: &str) -> bool {
        if let Some(dt) = parse_storage(time_str) {
            self.set_start_time(dt);
            true
        } else {
            false
        }
    }

    /// Sets the end time from a string using the storage format
    /// Returns true if parsing succeeded, false otherwise
    pub fn set_end_time_from_str(&mut self, time_str: &str) -> bool {
        if let Some(dt) = parse_storage(time_str) {
            self.set_end_time(dt);
            true
        } else {
            false
        }
    }

    /// Sets the duration from a string representing minutes
    /// Returns true if parsing succeeded, false otherwise
    pub fn set_duration_from_str(&mut self, minutes_str: &str) -> bool {
        if let Ok(minutes) = minutes_str.parse::<i64>() {
            self.set_duration(Duration::minutes(minutes));
            true
        } else {
            false
        }
    }

    /// Set the start time, transitioning to appropriate state
    pub fn set_start_time(&mut self, start_time: NaiveDateTime) {
        match self {
            TimeRange::Unspecified => {
                *self = TimeRange::UnspecifiedWithStart(start_time);
            }
            TimeRange::UnspecifiedWithDuration(duration) => {
                *self = TimeRange::Scheduled {
                    start_time,
                    duration: *duration,
                };
            }
            TimeRange::UnspecifiedWithStart(_) => {
                *self = TimeRange::UnspecifiedWithStart(start_time);
            }
            TimeRange::Scheduled { duration, .. } => {
                *self = TimeRange::Scheduled {
                    start_time,
                    duration: *duration,
                };
            }
        }
    }

    /// Set the duration, transitioning to appropriate state
    pub fn set_duration(&mut self, duration: Duration) {
        if duration < Duration::zero() {
            // Reject negative durations - transition to state without duration
            match self {
                TimeRange::UnspecifiedWithDuration(_) => {
                    *self = TimeRange::Unspecified;
                }
                TimeRange::Scheduled { start_time, .. } => {
                    // Transition to UnspecifiedWithStart, keeping the start time
                    *self = TimeRange::UnspecifiedWithStart(*start_time);
                }
                _ => {
                    // Keep as is
                }
            };
            return;
        }

        match self {
            TimeRange::Unspecified => {
                *self = TimeRange::UnspecifiedWithDuration(duration);
            }
            TimeRange::UnspecifiedWithDuration(_) => {
                *self = TimeRange::UnspecifiedWithDuration(duration);
            }
            TimeRange::UnspecifiedWithStart(start_time) => {
                *self = TimeRange::Scheduled {
                    start_time: *start_time,
                    duration,
                };
            }
            TimeRange::Scheduled { start_time, .. } => {
                *self = TimeRange::Scheduled {
                    start_time: *start_time,
                    duration,
                };
            }
        }
    }

    /// Set the end time, preserving duration if available
    pub fn set_end_time(&mut self, end_time: NaiveDateTime) {
        match self {
            TimeRange::Scheduled { duration, .. } => {
                // Preserve duration and adjust start time based on new end time
                let start_time = end_time - *duration;
                *self = TimeRange::Scheduled {
                    start_time,
                    duration: *duration,
                };
            }
            TimeRange::UnspecifiedWithDuration(duration) => {
                // Calculate start time from end time and duration
                let start_time = end_time - *duration;
                *self = TimeRange::Scheduled {
                    start_time,
                    duration: *duration,
                };
            }
            TimeRange::UnspecifiedWithStart(start_time) => {
                // Calculate duration from start and end times
                if *start_time < end_time {
                    *self = TimeRange::Scheduled {
                        start_time: *start_time,
                        duration: end_time - *start_time,
                    };
                } else {
                    // Invalid: end time before start time, transition to Unspecified
                    *self = TimeRange::Unspecified;
                }
            }
            _ => {
                // Can't set end time without start time, transition to Unspecified
                *self = TimeRange::Unspecified;
            }
        }
    }

    /// Set the end time, preserving start time (user-friendly for cosam-modify)
    /// This is typically what users want when setting an end time - keep the start fixed
    pub fn set_end_time_preserve_start(&mut self, end_time: NaiveDateTime) {
        match self {
            TimeRange::Scheduled { start_time, .. } => {
                // Preserve start time and calculate new duration
                if *start_time < end_time {
                    let duration = end_time - *start_time;
                    *self = TimeRange::Scheduled {
                        start_time: *start_time,
                        duration,
                    };
                } else {
                    // Invalid: end time before start time, transition to Unspecified
                    *self = TimeRange::Unspecified;
                }
            }
            TimeRange::UnspecifiedWithDuration(_duration) => {
                // Can't preserve start time since we don't have one
                // Fall back to regular behavior which will calculate start time
                self.set_end_time(end_time);
            }
            TimeRange::UnspecifiedWithStart(start_time) => {
                // This is the ideal case - we have start time, calculate duration
                if *start_time < end_time {
                    *self = TimeRange::Scheduled {
                        start_time: *start_time,
                        duration: end_time - *start_time,
                    };
                } else {
                    // Invalid: end time before start time, transition to Unspecified
                    *self = TimeRange::Unspecified;
                }
            }
            TimeRange::Unspecified => {
                // No start time available, can't preserve it
                // Could transition to UnspecifiedWithEnd if we add that variant
            }
        }
    }

    /// Check if this time range is scheduled (has both start time and valid duration)
    pub fn is_scheduled(&self) -> bool {
        match self {
            TimeRange::Scheduled {
                start_time: _,
                duration,
            } => {
                // Only consider scheduled if duration is positive
                duration.num_seconds() > 0
            }
            _ => false,
        }
    }

    /// Clear the start time, transitioning to appropriate state
    pub fn clear_start_time(&mut self) {
        match self {
            TimeRange::Scheduled { duration, .. } => {
                *self = TimeRange::UnspecifiedWithDuration(*duration);
            }
            TimeRange::UnspecifiedWithStart(_) => {
                *self = TimeRange::Unspecified;
            }
            TimeRange::UnspecifiedWithDuration(_) | TimeRange::Unspecified => {
                // No change needed
            }
        }
    }

    /// Clear the end time (effectively clear duration since end time is computed)
    pub fn clear_end_time(&mut self) {
        match self {
            TimeRange::Scheduled { start_time, .. } => {
                *self = TimeRange::UnspecifiedWithStart(*start_time);
            }
            TimeRange::UnspecifiedWithStart(_) | TimeRange::Unspecified => {
                // No change needed
            }
            TimeRange::UnspecifiedWithDuration(_) => {
                // No end time to clear in this state
            }
        }
    }

    /// Clear the duration, transitioning to appropriate state
    pub fn clear_duration(&mut self) {
        match self {
            TimeRange::Scheduled { start_time, .. } => {
                *self = TimeRange::UnspecifiedWithStart(*start_time);
            }
            TimeRange::UnspecifiedWithDuration(_) => {
                *self = TimeRange::Unspecified;
            }
            TimeRange::UnspecifiedWithStart(_) | TimeRange::Unspecified => {
                // No change needed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime_iso() {
        let dt = parse_datetime("2026-06-26T14:00:00").expect("ISO parse");
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-06-26 14:00");
    }

    #[test]
    fn test_parse_datetime_us() {
        let dt = parse_datetime("6/26/2026 2:00 PM").expect("US parse");
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-06-26 14:00");
    }

    #[test]
    fn test_parse_datetime_empty() {
        assert!(parse_datetime("").is_none());
        assert!(parse_datetime("   ").is_none());
    }

    #[test]
    fn test_parse_duration_str_hhmm() {
        assert_eq!(parse_duration_str("1:30"), Some(90));
        assert_eq!(parse_duration_str("2:00"), Some(120));
    }

    #[test]
    fn test_parse_duration_str_minutes() {
        assert_eq!(parse_duration_str("90"), Some(90));
        assert_eq!(parse_duration_str("60"), Some(60));
    }

    #[test]
    fn test_parse_duration_str_empty() {
        assert!(parse_duration_str("").is_none());
    }

    #[test]
    fn test_format_storage() {
        let dt = parse_datetime("2026-06-26T14:00:00").unwrap();
        assert_eq!(format_storage(dt), "2026-06-26T14:00:00");
    }

    #[test]
    fn test_format_roundtrip() {
        let original = "2026-06-26T14:30:00";
        let dt = parse_datetime(original).unwrap();
        assert_eq!(format_storage(dt), original);
    }

    // TimeRange tests
    #[test]
    fn test_timerange_new_scheduled_valid() {
        let start = parse_datetime("2026-06-26T14:00:00").unwrap();
        let duration = Duration::minutes(60);
        let timerange = TimeRange::new_scheduled(start, duration).unwrap();

        assert!(timerange.is_scheduled());
        assert_eq!(timerange.start_time(), Some(start));
        assert_eq!(timerange.duration(), Some(duration));
        assert_eq!(timerange.effective_end_time(), Some(start + duration));
    }

    #[test]
    fn test_timerange_new_scheduled_invalid_duration() {
        let start = parse_datetime("2026-06-26T14:00:00").unwrap();
        let duration = Duration::minutes(-10); // Negative duration

        let result = TimeRange::new_scheduled(start, duration);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Duration cannot be negative");
    }

    #[test]
    fn test_timerange_from_start_end_valid() {
        let start = parse_datetime("2026-06-26T14:00:00").unwrap();
        let end = parse_datetime("2026-06-26T15:00:00").unwrap();
        let timerange = TimeRange::from_start_end(start, end).unwrap();

        assert!(timerange.is_scheduled());
        assert_eq!(timerange.start_time(), Some(start));
        assert_eq!(timerange.duration(), Some(Duration::minutes(60)));
        assert_eq!(timerange.effective_end_time(), Some(end));
    }

    #[test]
    fn test_timerange_from_start_end_invalid() {
        let start = parse_datetime("2026-06-26T15:00:00").unwrap();
        let end = parse_datetime("2026-06-26T14:00:00").unwrap(); // End before start

        let result = TimeRange::from_start_end(start, end);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "End time must be after start time");
    }

    #[test]
    fn test_timerange_overlaps_with() {
        let start1 = parse_datetime("2026-06-26T14:00:00").unwrap();
        let end1 = parse_datetime("2026-06-26T15:00:00").unwrap();
        let range1 = TimeRange::from_start_end(start1, end1).unwrap();

        // Overlapping case
        let start2 = parse_datetime("2026-06-26T14:30:00").unwrap();
        let end2 = parse_datetime("2026-06-26T15:30:00").unwrap();
        let range2 = TimeRange::from_start_end(start2, end2).unwrap();
        assert!(range1.overlaps_with(&range2));
        assert!(range2.overlaps_with(&range1));

        // Non-overlapping case
        let start3 = parse_datetime("2026-06-26T16:00:00").unwrap();
        let end3 = parse_datetime("2026-06-26T17:00:00").unwrap();
        let range3 = TimeRange::from_start_end(start3, end3).unwrap();
        assert!(!range1.overlaps_with(&range3));
        assert!(!range3.overlaps_with(&range1));
    }

    #[test]
    fn test_timerange_contains_datetime() {
        let start = parse_datetime("2026-06-26T14:00:00").unwrap();
        let end = parse_datetime("2026-06-26T15:00:00").unwrap();
        let range = TimeRange::from_start_end(start, end).unwrap();

        // Contains case
        let contained = parse_datetime("2026-06-26T14:30:00").unwrap();
        assert!(range.contains_datetime(contained));

        // Edge cases
        assert!(range.contains_datetime(start)); // Start is inclusive
        assert!(!range.contains_datetime(end)); // End is exclusive

        // Not contained
        let outside = parse_datetime("2026-06-26T16:00:00").unwrap();
        assert!(!range.contains_datetime(outside));
    }

    #[test]
    fn test_timerange_string_methods() {
        let start = parse_datetime("2026-06-26T14:00:00").unwrap();
        let duration = Duration::minutes(90);
        let mut timerange = TimeRange::new_scheduled(start, duration).unwrap();

        // Test string getters
        assert_eq!(
            timerange.start_time_str(),
            Some("2026-06-26T14:00:00".to_string())
        );
        assert_eq!(
            timerange.end_time_str(),
            Some("2026-06-26T15:30:00".to_string())
        );
        assert_eq!(timerange.duration_minutes_str(), Some("90".to_string()));

        // Test string setters
        assert!(timerange.set_start_time_from_str("2026-06-26T16:00:00"));
        assert_eq!(
            timerange.start_time_str(),
            Some("2026-06-26T16:00:00".to_string())
        );

        assert!(timerange.set_end_time_from_str("2026-06-26T18:00:00"));
        assert_eq!(
            timerange.end_time_str(),
            Some("2026-06-26T18:00:00".to_string())
        );

        assert!(timerange.set_duration_from_str("120"));
        assert_eq!(timerange.duration_minutes_str(), Some("120".to_string()));

        // Test invalid string setters
        assert!(!timerange.set_start_time_from_str("invalid-date"));
        assert!(!timerange.set_end_time_from_str("invalid-date"));
        assert!(!timerange.set_duration_from_str("not-a-number"));
    }

    #[test]
    fn test_timerange_set_duration_validation() {
        let start = parse_datetime("2026-06-26T14:00:00").unwrap();
        let mut timerange = TimeRange::new_scheduled(start, Duration::minutes(60)).unwrap();

        // Valid duration
        timerange.set_duration(Duration::minutes(90));
        assert_eq!(timerange.duration(), Some(Duration::minutes(90)));
        assert!(timerange.is_scheduled());

        // Invalid duration (negative) - should transition to UnspecifiedWithStart
        timerange.set_duration(Duration::minutes(-10));
        assert_eq!(timerange.start_time(), Some(start));
        assert_eq!(timerange.duration(), None);
        assert!(!timerange.is_scheduled());
    }

    #[test]
    fn test_timerange_set_end_time_validation() {
        let start = parse_datetime("2026-06-26T14:00:00").unwrap();
        let mut timerange = TimeRange::new_scheduled(start, Duration::minutes(60)).unwrap();

        // Valid end time
        let valid_end = parse_datetime("2026-06-26T15:30:00").unwrap();
        timerange.set_end_time(valid_end);
        assert_eq!(timerange.effective_end_time(), Some(valid_end));
        assert!(timerange.is_scheduled());

        // Invalid end time (before start) - should preserve duration and adjust start time
        let invalid_end = parse_datetime("2026-06-26T13:00:00").unwrap();
        timerange.set_end_time(invalid_end);
        // Should preserve 60min duration and move start to 12:00
        assert_eq!(
            timerange.start_time(),
            Some(parse_datetime("2026-06-26T12:00:00").unwrap())
        );
        assert_eq!(timerange.effective_end_time(), Some(invalid_end));
        assert_eq!(timerange.duration(), Some(Duration::minutes(60)));
        assert!(timerange.is_scheduled()); // Still scheduled with preserved duration
    }

    #[test]
    fn test_timerange_is_scheduled_with_invalid_duration() {
        let start = parse_datetime("2026-06-26T14:00:00").unwrap();

        // Valid duration
        let valid_range = TimeRange::new_scheduled(start, Duration::minutes(60)).unwrap();
        assert!(valid_range.is_scheduled());

        // Zero duration
        let zero_range = TimeRange::new_scheduled(start, Duration::minutes(0)).unwrap();
        assert!(!zero_range.is_scheduled());

        // Negative duration (shouldn't be possible with new_scheduled, but test direct creation)
        let negative_range = TimeRange::Scheduled {
            start_time: start,
            duration: Duration::minutes(-10),
        };
        assert!(!negative_range.is_scheduled());
    }
}
