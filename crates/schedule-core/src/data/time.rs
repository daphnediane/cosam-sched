/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Date/time format constants and parsing helpers used throughout the codebase.

use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;

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

#[cfg(test)]
mod tests {
    /*
     * Copyright (c) 2026 Daphne Pfister
     * SPDX-License-Identifier: BSD-2-Clause
     * See LICENSE file for full license text
     */
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
}
