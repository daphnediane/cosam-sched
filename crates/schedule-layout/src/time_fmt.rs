/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared time formatting and loose parsing utilities.
//!
//! All schedule-layout modules that need to display times (grid labels, panel
//! headings, postcards, etc.) should use these functions to ensure consistent
//! formatting throughout the output.
//!
//! This module also provides loose/natural language datetime parsing for layout
//! configuration, resolving day names like "Friday" to actual dates based on
//! the schedule's date range.

use chrono::{Datelike, NaiveDate, NaiveDateTime};

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

// --- Loose time parsing ------------------------------------------------------

/// Parse a loose/natural language datetime string into an ISO 8601 datetime.
///
/// This resolves day names (Friday, Saturday, etc.) to actual dates based on
/// the schedule's date range. Supports various time formats.
///
/// # Examples
/// - `"fri 2 pm"` → `"2026-06-26T14:00:00"` (if Friday is within the range)
/// - `"Saturday Noon"` → `"2026-06-27T12:00:00"`
/// - `"June 26 8:30 pm"` → `"2026-06-26T20:30:00"`
/// - `"Sunday 9:00 AM"` → `"2026-06-28T09:00:00"`
///
/// # Arguments
/// * `text` - The natural language datetime string
/// * `schedule_start` - Start date of the schedule (ISO 8601: "2026-06-26")
/// * `schedule_end` - End date of the schedule (ISO 8601: "2026-06-28")
///
/// # Returns
/// Some(NaiveDateTime) if parsing succeeds, None otherwise.
#[must_use]
pub fn parse_loose_datetime(
    text: &str,
    schedule_start: &str,
    schedule_end: &str,
) -> Option<NaiveDateTime> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // First, try standard datetime formats (ISO 8601, etc.)
    if let Some(dt) = parse_datetime(text) {
        return Some(dt);
    }

    // Parse day name and time components
    let (day_match, time_part) = parse_day_and_time(text)?;

    // Find a matching date within the schedule range
    let target_date = resolve_day_to_date(day_match, schedule_start, schedule_end)?;

    // Parse the time part
    let time = parse_loose_time(time_part)?;

    Some(NaiveDateTime::new(target_date, time))
}

/// Whether a loose time string uses a bare weekday name (and therefore recurs weekly).
///
/// Returns `true` for inputs like `"Friday Noon"`, `"fri 2 pm"`, `"Saturday"`.
/// Returns `false` for pinned dates (`"June 12 Noon"`, `"2026-06-12T12:00"`, `"Noon"`).
#[must_use]
pub fn is_recurring(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    // If it parses as an ISO datetime, it's pinned.
    if parse_datetime(text).is_some() {
        return false;
    }
    // Recurring if the day component is a bare weekday name (not a full date or month+day).
    matches!(
        parse_day_and_time(text).map(|(d, _)| d),
        Some(DayMatch::DayName(_))
    )
}

/// A single resolved slot occurrence produced by expanding a `CustomTimeSlot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotOccurrence {
    /// Rendered label (template substitutions applied).
    pub label: String,
    /// Start datetime (ISO 8601 storage format).
    pub start: NaiveDateTime,
    /// End datetime, if set. Panels at or after this are excluded.
    pub end: Option<NaiveDateTime>,
}

/// Expand a single TOML slot definition into one or more resolved occurrences.
///
/// For **pinned** slots (month+day or full date prefix), returns exactly one item.
/// For **recurring** slots (bare weekday name), returns one item per matching weekday
/// in the range `[schedule_start, schedule_end]`.
///
/// The `end_str` is the optional `end =` field from the TOML slot. For recurring
/// slots it is re-anchored to the same week as each occurrence.
///
/// Label templates: `{date}` → formatted occurrence date (e.g. "Jun 12"),
/// `{time}` → formatted occurrence start time (e.g. "Noon").
#[must_use]
pub fn expand_slot(
    label_template: &str,
    time_str: &str,
    end_str: Option<&str>,
    schedule_start: &str,
    schedule_end: &str,
) -> Vec<SlotOccurrence> {
    let time_str = time_str.trim();
    if time_str.is_empty() {
        return vec![];
    }

    let start_date = match parse_date_only(schedule_start) {
        Some(d) => d,
        None => return vec![],
    };
    let end_date = match parse_date_only(schedule_end) {
        Some(d) => d,
        None => return vec![],
    };

    // Collect all occurrence dates: one for pinned, one-per-week for recurring.
    let (day_match, time_part) = match parse_day_and_time(time_str) {
        Some(p) => p,
        None => return vec![],
    };
    let Some(time) = parse_loose_time(time_part) else {
        return vec![];
    };

    let occurrence_dates: Vec<NaiveDate> = match day_match {
        DayMatch::DayName(day_name) => {
            let Some(target_weekday) = parse_weekday(day_name) else {
                return vec![];
            };
            // Walk from start_date to end_date, collecting every matching weekday.
            let mut dates = vec![];
            let mut cur = start_date;
            while cur <= end_date {
                if cur.weekday() == target_weekday {
                    dates.push(cur);
                }
                cur = match cur.succ_opt() {
                    Some(d) => d,
                    None => break,
                };
            }
            dates
        }
        _ => {
            // Pinned: resolve once.
            match resolve_day_to_date(day_match, schedule_start, schedule_end) {
                Some(d) => vec![d],
                None => return vec![],
            }
        }
    };

    occurrence_dates
        .into_iter()
        .map(|date| {
            let start_dt = NaiveDateTime::new(date, time);

            // Resolve end time, re-anchoring to this occurrence's date if recurring.
            let end_dt = end_str.and_then(|e| {
                let e = e.trim();
                // If the end string has no day component (e.g. "3 pm") anchor to
                // the same date as the start. Otherwise parse normally within range.
                let (end_day, end_time_part) = parse_day_and_time(e)?;
                let end_time = parse_loose_time(end_time_part)?;
                let end_date = match end_day {
                    DayMatch::Unspecified => date, // no day prefix → same date as start
                    DayMatch::DayName(name) => {
                        // Re-anchor: find the occurrence of that weekday in the same week as start.
                        let target = parse_weekday(name)?;
                        // Walk forward/backward up to 6 days from the start date.
                        let mut d = date;
                        for _ in 0..7 {
                            if d.weekday() == target {
                                break;
                            }
                            d = d.succ_opt()?;
                        }
                        d
                    }
                    _ => resolve_day_to_date(end_day, schedule_start, schedule_end)?,
                };
                Some(NaiveDateTime::new(end_date, end_time))
            });

            // Render label: substitute {date} and {time}
            let date_str = date.format("%b %-d").to_string(); // e.g. "Jun 12"
            let time_str = format_time(&start_dt.format("%Y-%m-%dT%H:%M:%S").to_string());
            let label = label_template
                .replace("{date}", &date_str)
                .replace("{time}", &time_str);

            SlotOccurrence {
                label,
                start: start_dt,
                end: end_dt,
            }
        })
        .collect()
}

/// Parse standard datetime formats.
fn parse_datetime(text: &str) -> Option<NaiveDateTime> {
    // ISO 8601 with T
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    // ISO with space
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    None
}

/// Result of parsing day and time from a loose datetime string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayMatch<'a> {
    /// Day name like "Friday", "fri"
    DayName(&'a str),
    /// Month and day like "June 26", "6/26"
    MonthDay { month: u32, day: u32 },
    /// Full date like "2026-06-26"
    FullDate(NaiveDate),
    /// No day specified - use first schedule date
    Unspecified,
}

/// Parse the day and time components from a loose datetime string.
fn parse_day_and_time(text: &str) -> Option<(DayMatch<'_>, &str)> {
    // Try full date prefix first (YYYY-MM-DD or YYYY/MM/DD)
    let re_full_date = regex::Regex::new(r"^(\d{4})[-/](\d{1,2})[-/](\d{1,2})\s+(.*)$").ok()?;
    if let Some(caps) = re_full_date.captures(text) {
        let year: i32 = caps[1].parse().ok()?;
        let month: u32 = caps[2].parse().ok()?;
        let day: u32 = caps[3].parse().ok()?;
        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        let time_part = caps.get(4)?.as_str().trim();
        return Some((DayMatch::FullDate(date), time_part));
    }

    // Try month name + day prefix (June 26, Jun 26)
    let re_month_day = regex::Regex::new(
        r"^(?i)(January|Jan|February|Feb|March|Mar|April|Apr|May|June|Jun|July|Jul|August|Aug|September|Sep|Sept|October|Oct|November|Nov|December|Dec)\s+(\d{1,2})(?:st|nd|rd|th)?\s+(.*)$",
    )
    .ok()?;
    if let Some(caps) = re_month_day.captures(text) {
        let month = parse_month_name(&caps[1])?;
        let day: u32 = caps[2].parse().ok()?;
        let time_part = caps.get(3)?.as_str().trim();
        return Some((DayMatch::MonthDay { month, day }, time_part));
    }

    // Try numeric M/D prefix (6/26)
    let re_numeric_md = regex::Regex::new(r"^(\d{1,2})/(\d{1,2})\s+(.*)$").ok()?;
    if let Some(caps) = re_numeric_md.captures(text) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let time_part = caps.get(3)?.as_str().trim();
        return Some((DayMatch::MonthDay { month, day }, time_part));
    }

    // Try day name prefix (Friday, fri, Sat, Saturday) — time part is optional.
    // A bare day name like "Thursday" with no time defaults to midnight (start of day).
    let re_day_name = regex::Regex::new(
        r"^(?i)(Monday|Mon|Tuesday|Tue|Tues|Wednesday|Wed|Thursday|Thu|Thurs|Friday|Fri|Saturday|Sat|Sunday|Sun)\s*(.*)$",
    )
    .ok()?;
    if let Some(caps) = re_day_name.captures(text) {
        let day_name = caps.get(1)?.as_str();
        let time_part = caps.get(2)?.as_str().trim();
        return Some((DayMatch::DayName(day_name), time_part));
    }

    // No day prefix - assume time only and use first schedule date
    Some((DayMatch::Unspecified, text))
}

/// Parse a month name into its numeric value.
fn parse_month_name(name: &str) -> Option<u32> {
    let name_lower = name.to_lowercase();
    match name_lower.as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

/// Parse a weekday name into its chrono weekday.
fn parse_weekday(name: &str) -> Option<chrono::Weekday> {
    let name_lower = name.to_lowercase();
    match name_lower.as_str() {
        "monday" | "mon" => Some(chrono::Weekday::Mon),
        "tuesday" | "tue" | "tues" => Some(chrono::Weekday::Tue),
        "wednesday" | "wed" => Some(chrono::Weekday::Wed),
        "thursday" | "thu" | "thurs" => Some(chrono::Weekday::Thu),
        "friday" | "fri" => Some(chrono::Weekday::Fri),
        "saturday" | "sat" => Some(chrono::Weekday::Sat),
        "sunday" | "sun" => Some(chrono::Weekday::Sun),
        _ => None,
    }
}

/// Resolve a day specification to an actual date within the schedule range.
fn resolve_day_to_date(
    day_match: DayMatch,
    schedule_start: &str,
    schedule_end: &str,
) -> Option<NaiveDate> {
    let start_date = parse_date_only(schedule_start)?;
    let end_date = parse_date_only(schedule_end)?;

    match day_match {
        DayMatch::FullDate(date) => Some(date),
        DayMatch::Unspecified => Some(start_date),
        DayMatch::MonthDay { month, day } => {
            // Infer year from schedule start date
            let year = start_date.year();
            NaiveDate::from_ymd_opt(year, month, day)
        }
        DayMatch::DayName(day_name) => {
            let target_weekday = parse_weekday(day_name)?;

            // Search through the date range to find a matching weekday
            let mut current = start_date;
            while current <= end_date {
                if current.weekday() == target_weekday {
                    return Some(current);
                }
                current = current.succ_opt()?;
            }
            None
        }
    }
}

/// Parse just the date portion from an ISO 8601 date string.
fn parse_date_only(date_str: &str) -> Option<NaiveDate> {
    // Handle both "2026-06-26" and "2026-06-26T14:00:00" formats
    let date_part = date_str.split('T').next()?;
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
}

/// Parse a loose time expression into a chrono::NaiveTime.
///
/// Supports: "2 pm", "2:30 PM", "14:00", "noon", "midnight"
///
/// # Note on "midnight"
/// "midnight" is parsed as 12:00 AM (00:00) - the start of the day, not the end.
/// This means "Friday midnight" refers to the very beginning of Friday.
/// For the end of Friday (just before Saturday), use "11:59 PM" or "Friday 11:59 PM".
fn parse_loose_time(time_str: &str) -> Option<chrono::NaiveTime> {
    use chrono::NaiveTime;

    let time_str = time_str.trim().to_lowercase();

    // Empty string — bare day name with no time → midnight (start of day)
    if time_str.is_empty() {
        return NaiveTime::from_hms_opt(0, 0, 0);
    }

    // Special cases
    if time_str == "noon" || time_str == "12 noon" {
        return NaiveTime::from_hms_opt(12, 0, 0);
    }
    if time_str == "midnight" {
        return NaiveTime::from_hms_opt(0, 0, 0);
    }

    // Try 24-hour format (14:00, 14:00:00)
    if let Ok(time) = NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
        return Some(time);
    }
    if let Ok(time) = NaiveTime::parse_from_str(&time_str, "%H:%M") {
        return Some(time);
    }

    // Parse 12-hour format with AM/PM
    let re_12h = regex::Regex::new(r"^(\d{1,2}):?(\d{2})?\s*(am|pm)$").ok()?;
    if let Some(caps) = re_12h.captures(&time_str) {
        let mut hour: u32 = caps[1].parse().ok()?;
        let minute: u32 = caps
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);
        let ampm = caps.get(3)?.as_str();

        match ampm {
            "am" => {
                if hour == 12 {
                    hour = 0;
                }
            }
            "pm" => {
                if hour < 12 {
                    hour += 12;
                }
            }
            _ => return None,
        }
        return NaiveTime::from_hms_opt(hour, minute, 0);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

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

    // --- Loose parsing tests -------------------------------------------------

    #[test]
    fn test_parse_loose_time_noon() {
        let time = parse_loose_time("noon").unwrap();
        assert_eq!(time.hour(), 12);
        assert_eq!(time.minute(), 0);
    }

    #[test]
    fn test_parse_loose_time_midnight() {
        let time = parse_loose_time("midnight").unwrap();
        assert_eq!(time.hour(), 0);
        assert_eq!(time.minute(), 0);
    }

    #[test]
    fn test_parse_loose_time_pm() {
        let time = parse_loose_time("2 pm").unwrap();
        assert_eq!(time.hour(), 14);
        assert_eq!(time.minute(), 0);
    }

    #[test]
    fn test_parse_loose_time_24h() {
        let time = parse_loose_time("14:30").unwrap();
        assert_eq!(time.hour(), 14);
        assert_eq!(time.minute(), 30);
    }

    #[test]
    fn test_parse_loose_datetime_iso() {
        let dt = parse_loose_datetime("2026-06-26T14:00:00", "2026-06-26", "2026-06-28").unwrap();
        assert_eq!(dt.date().to_string(), "2026-06-26");
        assert_eq!(dt.time().hour(), 14);
    }

    #[test]
    fn test_parse_loose_datetime_day_name() {
        // Friday June 26, 2026 - search through Tue-Sun range
        let dt = parse_loose_datetime("fri 2 pm", "2026-06-23", "2026-06-28").unwrap();
        assert_eq!(dt.date().to_string(), "2026-06-26");
        assert_eq!(dt.date().weekday(), chrono::Weekday::Fri);
        assert_eq!(dt.time().hour(), 14);
    }

    #[test]
    fn test_parse_loose_datetime_day_name_not_in_discrete_dates() {
        // Wednesday isn't in the list ["2026-06-24", "2026-06-26"] but is in the range
        let dt = parse_loose_datetime("wed 10 am", "2026-06-23", "2026-06-28").unwrap();
        assert_eq!(dt.date().to_string(), "2026-06-24"); // Wednesday June 24
        assert_eq!(dt.date().weekday(), chrono::Weekday::Wed);
    }

    #[test]
    fn test_parse_loose_datetime_month_day() {
        let dt = parse_loose_datetime("June 26 8:30 pm", "2026-06-26", "2026-06-28").unwrap();
        assert_eq!(dt.date().month(), 6);
        assert_eq!(dt.date().day(), 26);
        assert_eq!(dt.time().hour(), 20);
        assert_eq!(dt.time().minute(), 30);
    }

    #[test]
    fn test_parse_month_name() {
        assert_eq!(parse_month_name("January"), Some(1));
        assert_eq!(parse_month_name("Jan"), Some(1));
        assert_eq!(parse_month_name("jun"), Some(6));
        assert_eq!(parse_month_name("Dec"), Some(12));
        assert_eq!(parse_month_name("invalid"), None);
    }

    #[test]
    fn test_parse_weekday() {
        assert_eq!(parse_weekday("Friday"), Some(chrono::Weekday::Fri));
        assert_eq!(parse_weekday("fri"), Some(chrono::Weekday::Fri));
        assert_eq!(parse_weekday("Sat"), Some(chrono::Weekday::Sat));
        assert_eq!(parse_weekday("invalid"), None);
    }

    // --- expand_slot tests ---------------------------------------------------

    #[test]
    fn test_expand_slot_pinned() {
        // Pinned to a specific date: only one occurrence
        let occs = expand_slot(
            "Lunch",
            "June 25 Noon",
            Some("3 pm"),
            "2026-06-25",
            "2026-06-28",
        );
        assert_eq!(occs.len(), 1);
        assert_eq!(occs[0].label, "Lunch");
        assert_eq!(occs[0].start.time().hour(), 12);
        assert_eq!(occs[0].end.unwrap().time().hour(), 15);
    }

    #[test]
    fn test_expand_slot_recurring_fridays() {
        // Jun 11 to Jul 8: Fridays are Jun 12, Jun 19, Jun 26, Jul 3
        let occs = expand_slot(
            "{date}",
            "Friday Noon",
            Some("Friday 3 pm"),
            "2026-06-11",
            "2026-07-08",
        );
        assert_eq!(occs.len(), 4);
        // Check first occurrence: Jun 12 (Friday)
        assert_eq!(occs[0].start.date().to_string(), "2026-06-12");
        assert_eq!(occs[0].start.time().hour(), 12);
        assert_eq!(occs[0].end.unwrap().time().hour(), 15);
        assert_eq!(occs[0].label, "Jun 12");
        // Check last: Jul 3
        assert_eq!(occs[3].start.date().to_string(), "2026-07-03");
        assert_eq!(occs[3].label, "Jul 3");
    }

    #[test]
    fn test_expand_slot_end_same_day_no_prefix() {
        // "3 pm" with no day prefix anchors to same date as start
        let occs = expand_slot(
            "{date}",
            "Friday Noon",
            Some("3 pm"),
            "2026-06-11",
            "2026-07-08",
        );
        assert_eq!(occs.len(), 4);
        for occ in &occs {
            // end is on same date as start
            let end = occ.end.unwrap();
            assert_eq!(end.date(), occ.start.date());
            assert_eq!(end.time().hour(), 15);
        }
    }

    #[test]
    fn test_expand_slot_empty_label_excluded() {
        // An empty label is still returned (caller filters)
        let occs = expand_slot("", "Friday 3 pm", None, "2026-06-11", "2026-07-08");
        assert_eq!(occs.len(), 4);
        for occ in &occs {
            assert!(occ.label.is_empty());
        }
    }

    #[test]
    fn test_bare_day_name_resolves_to_midnight() {
        // "Thursday" with no time → Thursday midnight (start of day)
        // Jun 25 2026 is a Thursday; range Thu–Sun
        let dt = parse_loose_datetime("Thursday", "2026-06-25", "2026-06-28").unwrap();
        assert_eq!(dt.date().weekday(), chrono::Weekday::Thu);
        assert_eq!(dt.time().hour(), 0);
        assert_eq!(dt.time().minute(), 0);
    }

    #[test]
    fn test_bare_day_name_expand_slot() {
        // "Sunday" bare → one occurrence, midnight
        // Jun 28 2026 is a Sunday
        let occs = expand_slot("Sunday", "Sunday", None, "2026-06-25", "2026-06-28");
        assert_eq!(occs.len(), 1);
        assert_eq!(occs[0].start.date().weekday(), chrono::Weekday::Sun);
        assert_eq!(occs[0].start.time().hour(), 0);
    }

    #[test]
    fn test_is_recurring() {
        assert!(is_recurring("Friday Noon"));
        assert!(is_recurring("fri 2 pm"));
        assert!(!is_recurring("June 12 Noon"));
        assert!(!is_recurring("2026-06-12T12:00:00"));
        assert!(!is_recurring("Noon")); // Unspecified day → not recurring
    }

    #[test]
    fn test_label_template_date_and_time() {
        let occs = expand_slot(
            "Hangout {date} at {time}",
            "Friday Noon",
            None,
            "2026-06-11",
            "2026-06-18", // Jun 18 is Thursday, so only one Friday (Jun 12) in range
        );
        assert_eq!(occs.len(), 1);
        assert_eq!(occs[0].label, "Hangout Jun 12 at Noon");
    }
}
