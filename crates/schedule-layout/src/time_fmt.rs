/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared time formatting utilities.
//!
//! All schedule-layout modules that need to display times (grid labels, panel
//! headings, postcards, etc.) should use these functions to ensure consistent
//! formatting throughout the output.

use chrono::NaiveDate;

/// Format a datetime or time string's time component as `"5 PM"`, `"Noon"`, etc.
///
/// Accepts either a full ISO datetime (`"2026-06-26T14:30:00"`) or just a time
/// portion (`"14:30"`). Returns an empty string if parsing fails.
///
/// Special cases:
/// - 12:00 PM → `"Noon"`
/// - 12:00 AM → `"Midnight"`
pub fn format_time(datetime_str: &str) -> String {
    let time_part = datetime_str.get(11..).unwrap_or(datetime_str);
    let parts: Vec<&str> = time_part.splitn(2, ':').collect();
    if parts.len() < 2 {
        return String::new();
    }
    let hour: u32 = parts[0].parse().unwrap_or(0);
    let min: u32 = parts[1].get(..2).unwrap_or("0").parse().unwrap_or(0);
    let (h12, suffix) = if hour == 0 {
        (12u32, "AM")
    } else if hour < 12 {
        (hour, "AM")
    } else if hour == 12 {
        (12u32, "PM")
    } else {
        (hour - 12, "PM")
    };
    if h12 == 12 && min == 0 {
        return if suffix == "PM" {
            "Noon".to_string()
        } else {
            "Midnight".to_string()
        };
    }
    if min == 0 {
        format!("{} {}", h12, suffix)
    } else {
        format!("{}:{:02} {}", h12, min, suffix)
    }
}

/// Split a time into `(hour_part, suffix_part)` for widget-style aligned display.
///
/// Mirrors `formatTimeSplit` in `cosam-calendar.js`:
/// - The **hour part** is right-aligned (just the digit(s), e.g. `"2"` or `"10"`).
/// - The **suffix part** is left-aligned (` AM` / ` PM` for on-the-hour, or
///   `:MM AM`/`:MM PM` for minute-precision times).
/// - Special cases `Noon` and `Midnight` return `("Noon"/"Midnight", "")` so the
///   caller can span both columns and center the label.
///
/// Returns `("", "")` if the input cannot be parsed.
pub fn format_time_split(datetime_str: &str) -> (String, String) {
    let time_part = datetime_str.get(11..).unwrap_or(datetime_str);
    let parts: Vec<&str> = time_part.splitn(2, ':').collect();
    if parts.len() < 2 {
        return (String::new(), String::new());
    }
    let hour: u32 = parts[0].parse().unwrap_or(0);
    let min: u32 = parts[1].get(..2).unwrap_or("0").parse().unwrap_or(0);

    // Special whole-hour cases
    if hour == 0 && min == 0 {
        return ("Midnight".to_string(), String::new());
    }
    if hour == 12 && min == 0 {
        return ("Noon".to_string(), String::new());
    }

    let (h12, ampm) = if hour == 0 {
        (12u32, "AM")
    } else if hour < 12 {
        (hour, "AM")
    } else if hour == 12 {
        (12u32, "PM")
    } else {
        (hour - 12, "PM")
    };

    let hour_str = h12.to_string();
    let suffix = if min == 0 {
        format!("\u{00A0}{ampm}") // non-breaking space before AM/PM, matching widget
    } else {
        // Non-breaking space here too so ":30 PM" never wraps to two lines in a
        // narrow column (e.g. the panel-list time grid).
        format!(":{:02}\u{00A0}{ampm}", min)
    };
    (hour_str, suffix)
}

/// Format start–end as `"5 PM – 6 PM"` or `"5:30 PM – 7 PM"`.
///
/// Returns only the start time if end is `None`, or empty if both are `None`.
pub fn format_time_range(start: Option<&str>, end: Option<&str>) -> String {
    match (start, end) {
        (Some(s), Some(e)) => format!("{} – {}", format_time(s), format_time(e)),
        (Some(s), None) => format_time(s),
        _ => String::new(),
    }
}

/// Format a datetime as `"Saturday 2 PM"` for cross-reference labels.
///
/// When the reference is on the same day as `current_day_date`, the weekday
/// is omitted (returns just the time).
pub fn format_weekday_time(datetime_str: &str, current_day_date: &str) -> String {
    let date_str = datetime_str.get(..10).unwrap_or("");
    let time_str = format_time(datetime_str);

    if date_str.is_empty() || date_str == current_day_date {
        return time_str;
    }

    let weekday = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|d| d.format("%A").to_string())
        .unwrap_or_default();

    if weekday.is_empty() {
        time_str
    } else {
        format!("{} {}", weekday, time_str)
    }
}

/// Build a time-slot heading: `"Friday Noon"`, `"Friday 1:30 PM"`, etc.
///
/// When `day_label` is empty the time is returned on its own.
pub fn format_slot_heading(day_label: &str, time_key: &str) -> String {
    let t = format_time(time_key);
    if t.is_empty() {
        return String::new();
    }
    if day_label.is_empty() {
        t
    } else {
        format!("{} {}", day_label, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_split_on_hour() {
        let (h, s) = format_time_split("2026-06-26T14:00:00");
        assert_eq!(h, "2");
        assert_eq!(s, "\u{00A0}PM");
    }

    #[test]
    fn test_format_time_split_with_minutes() {
        let (h, s) = format_time_split("2026-06-26T14:30:00");
        assert_eq!(h, "2");
        assert_eq!(s, ":30\u{00A0}PM");
    }

    #[test]
    fn test_format_time_split_double_digit_hour() {
        let (h, s) = format_time_split("2026-06-26T22:00:00");
        assert_eq!(h, "10");
        assert_eq!(s, "\u{00A0}PM");
    }

    #[test]
    fn test_format_time_split_noon() {
        let (h, s) = format_time_split("2026-06-26T12:00:00");
        assert_eq!(h, "Noon");
        assert_eq!(s, "");
    }

    #[test]
    fn test_format_time_split_midnight() {
        let (h, s) = format_time_split("2026-06-26T00:00:00");
        assert_eq!(h, "Midnight");
        assert_eq!(s, "");
    }

    #[test]
    fn test_format_time_noon() {
        assert_eq!(format_time("2026-06-26T12:00:00"), "Noon");
        assert_eq!(format_time("2026-06-26T12:00"), "Noon");
    }

    #[test]
    fn test_format_time_midnight() {
        assert_eq!(format_time("2026-06-26T00:00:00"), "Midnight");
    }

    #[test]
    fn test_format_time_pm() {
        assert_eq!(format_time("2026-06-26T13:00:00"), "1 PM");
        assert_eq!(format_time("2026-06-26T17:00"), "5 PM");
    }

    #[test]
    fn test_format_time_am() {
        assert_eq!(format_time("2026-06-26T09:00:00"), "9 AM");
    }

    #[test]
    fn test_format_time_half_hour() {
        assert_eq!(format_time("2026-06-26T14:30:00"), "2:30 PM");
        assert_eq!(format_time("2026-06-26T09:15"), "9:15 AM");
    }

    #[test]
    fn test_format_time_range_both() {
        assert_eq!(
            format_time_range(Some("2026-06-25T17:00:00"), Some("2026-06-25T18:00:00")),
            "5 PM – 6 PM"
        );
        assert_eq!(
            format_time_range(Some("2026-06-25T21:30:00"), Some("2026-06-25T23:00:00")),
            "9:30 PM – 11 PM"
        );
    }

    #[test]
    fn test_format_time_range_start_only() {
        assert_eq!(format_time_range(Some("2026-06-25T12:00:00"), None), "Noon");
    }

    #[test]
    fn test_format_time_range_none() {
        assert_eq!(format_time_range(None, None), "");
    }

    #[test]
    fn test_format_weekday_time_cross_day() {
        assert_eq!(
            format_weekday_time("2026-06-27T14:00:00", "2026-06-26"),
            "Saturday 2 PM"
        );
    }

    #[test]
    fn test_format_weekday_time_same_day() {
        assert_eq!(
            format_weekday_time("2026-06-27T14:00:00", "2026-06-27"),
            "2 PM"
        );
    }

    #[test]
    fn test_format_slot_heading_with_day() {
        assert_eq!(
            format_slot_heading("Friday", "2026-06-27T14:00:00"),
            "Friday 2 PM"
        );
    }

    #[test]
    fn test_format_slot_heading_no_day() {
        assert_eq!(format_slot_heading("", "2026-06-27T12:00:00"), "Noon");
    }
}
