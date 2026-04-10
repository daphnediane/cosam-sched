/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Time range representation and datetime/duration parsing helpers.
//!
//! The core type is [`TimeRange`], a 6-variant enum that records exactly which
//! pair of values (start+duration or start+end) is canonical, following the
//! invariant that **setting end time or duration never adjusts start time**.

mod range;

use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::fmt;

// --- Format constants -------------------------------------------------------

/// Internal ISO-8601 storage format for timestamps (no timezone).
pub const STORAGE_FMT: &str = "%Y-%m-%dT%H:%M:%S";

// --- Parsing helpers --------------------------------------------------------

/// Parse a datetime string in any format found in schedule spreadsheets.
///
/// Accepted formats (in order):
/// - ISO 8601 with T: `2026-06-26T14:00:00`
/// - ISO with space:  `2026-06-26 14:00:00`
/// - US M/D/YYYY H:MM [AM/PM]: `6/26/2026 2:00 PM`
pub fn parse_datetime(text: &str) -> Option<NaiveDateTime> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, STORAGE_FMT) {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }
    let re_us = regex::Regex::new(
        r"^(\d{1,2})/(\d{1,2})/(\d{4})\s+(\d{1,2}):(\d{2})(?::(\d{2}))?\s*(AM|PM)?$",
    )
    .ok()?;
    if let Some(caps) = re_us.captures(text) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let year: i32 = caps[3].parse().ok()?;
        let mut hour: u32 = caps[4].parse().ok()?;
        let minute: u32 = caps[5].parse().ok()?;
        let second: u32 = caps.get(6).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
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

/// Format a [`NaiveDateTime`] using the internal storage format.
pub fn format_storage(dt: NaiveDateTime) -> String {
    dt.format(STORAGE_FMT).to_string()
}

/// Parse a duration string into a [`Duration`].
///
/// Accepted formats:
/// - `H:MM` or `HH:MM` — hours and minutes
/// - Plain integer — minutes
pub fn parse_duration(text: &str) -> Option<Duration> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let re_hm = regex::Regex::new(r"^(\d+):(\d{1,2})$").ok()?;
    if let Some(caps) = re_hm.captures(text) {
        let hours: i64 = caps[1].parse().ok()?;
        let minutes: i64 = caps[2].parse().ok()?;
        return Some(Duration::minutes(hours * 60 + minutes));
    }
    if let Ok(minutes) = text.parse::<i64>() {
        return Some(Duration::minutes(minutes));
    }
    None
}

// --- Serde helper -----------------------------------------------------------

/// Private helper struct for round-tripping [`TimeRange`] through serde.
///
/// Serializes as an object with up to 3 optional fields:
/// ```json
/// {"start_time": "2026-06-26T14:00:00", "duration": 60}
/// ```
/// The presence/absence of fields uniquely determines the variant.
#[derive(Debug, Default, Serialize, Deserialize)]
struct TimeRangeHelper {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end_time: Option<String>,
    /// Duration stored as whole minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duration: Option<i64>,
}

impl From<TimeRange> for TimeRangeHelper {
    fn from(tr: TimeRange) -> Self {
        match tr {
            TimeRange::Unspecified => Self::default(),
            TimeRange::UnspecifiedWithDuration(d) => Self {
                duration: Some(d.num_minutes()),
                ..Default::default()
            },
            TimeRange::UnspecifiedWithEnd(e) => Self {
                end_time: Some(format_storage(e)),
                ..Default::default()
            },
            TimeRange::UnspecifiedWithStart(s) => Self {
                start_time: Some(format_storage(s)),
                ..Default::default()
            },
            TimeRange::ScheduledWithDuration { start_time, duration } => Self {
                start_time: Some(format_storage(start_time)),
                duration: Some(duration.num_minutes()),
                ..Default::default()
            },
            TimeRange::ScheduledWithEnd { start_time, end_time } => Self {
                start_time: Some(format_storage(start_time)),
                end_time: Some(format_storage(end_time)),
                ..Default::default()
            },
        }
    }
}

impl From<TimeRangeHelper> for TimeRange {
    fn from(h: TimeRangeHelper) -> Self {
        let start = h.start_time.as_deref().and_then(parse_datetime);
        let end = h.end_time.as_deref().and_then(parse_datetime);
        let dur = h.duration.map(Duration::minutes);
        match (start, end, dur) {
            (None, None, None) => TimeRange::Unspecified,
            (None, None, Some(d)) => TimeRange::UnspecifiedWithDuration(d),
            (None, Some(e), _) => TimeRange::UnspecifiedWithEnd(e),
            (Some(s), None, None) => TimeRange::UnspecifiedWithStart(s),
            (Some(s), None, Some(d)) => {
                TimeRange::ScheduledWithDuration { start_time: s, duration: d }
            }
            (Some(s), Some(e), _) => TimeRange::ScheduledWithEnd { start_time: s, end_time: e },
        }
    }
}

// --- TimeRange enum ---------------------------------------------------------

/// Represents all possible time-slot states for a panel.
///
/// The variant encodes exactly which pair of values is canonical:
///
/// | Variant                | Canonical       | Computed  |
/// |------------------------|-----------------|-----------|
/// | `ScheduledWithDuration`| start + duration| end       |
/// | `ScheduledWithEnd`     | start + end     | duration  |
///
/// # Invariant
///
/// Setting end time or duration **never** adjusts start time.
/// Setting start time keeps whichever end/duration variant was active,
/// unless the new start would be ≥ the stored end time (in which case
/// the state falls back to `UnspecifiedWithStart`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(from = "TimeRangeHelper", into = "TimeRangeHelper")]
pub enum TimeRange {
    /// No timing information has been specified.
    #[default]
    Unspecified,
    /// A duration is known but no start time yet.
    UnspecifiedWithDuration(Duration),
    /// An end time is known but no start time yet.
    UnspecifiedWithEnd(NaiveDateTime),
    /// A start time is known but no duration or end time yet.
    UnspecifiedWithStart(NaiveDateTime),
    /// Fully scheduled: start + duration are canonical, end is computed.
    ScheduledWithDuration {
        start_time: NaiveDateTime,
        duration: Duration,
    },
    /// Fully scheduled: start + end are canonical, duration is computed.
    ScheduledWithEnd {
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    },
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeRange::Unspecified => write!(f, "Unspecified"),
            TimeRange::UnspecifiedWithDuration(d) => {
                write!(f, "Unspecified ({} min)", d.num_minutes())
            }
            TimeRange::UnspecifiedWithEnd(e) => {
                write!(f, "Unspecified (ends {})", e.format("%Y-%m-%d %H:%M"))
            }
            TimeRange::UnspecifiedWithStart(s) => {
                write!(f, "Unspecified (starts {})", s.format("%Y-%m-%d %H:%M"))
            }
            TimeRange::ScheduledWithDuration { start_time, duration } => {
                write!(f, "{} ({} min)", start_time.format("%Y-%m-%d %H:%M"), duration.num_minutes())
            }
            TimeRange::ScheduledWithEnd { start_time, end_time } => {
                write!(
                    f,
                    "{} to {}",
                    start_time.format("%Y-%m-%d %H:%M"),
                    end_time.format("%Y-%m-%d %H:%M")
                )
            }
        }
    }
}

// --- Read accessors ---------------------------------------------------------

impl TimeRange {
    /// Returns `true` if both a start time and at least one of (duration, end
    /// time) are known.
    #[must_use]
    pub fn is_scheduled(&self) -> bool {
        matches!(
            self,
            TimeRange::ScheduledWithDuration { .. } | TimeRange::ScheduledWithEnd { .. }
        )
    }

    /// Returns the start time if one has been set.
    #[must_use]
    pub fn start_time(&self) -> Option<NaiveDateTime> {
        match self {
            TimeRange::UnspecifiedWithStart(s) => Some(*s),
            TimeRange::ScheduledWithDuration { start_time, .. } => Some(*start_time),
            TimeRange::ScheduledWithEnd { start_time, .. } => Some(*start_time),
            _ => None,
        }
    }

    /// Returns the effective end time if it can be determined.
    ///
    /// For `ScheduledWithDuration` this is `start + duration`;
    /// for `ScheduledWithEnd` / `UnspecifiedWithEnd` it is the stored value.
    #[must_use]
    pub fn end_time(&self) -> Option<NaiveDateTime> {
        match self {
            TimeRange::UnspecifiedWithEnd(e) => Some(*e),
            TimeRange::ScheduledWithEnd { end_time, .. } => Some(*end_time),
            TimeRange::ScheduledWithDuration { start_time, duration } => {
                Some(*start_time + *duration)
            }
            _ => None,
        }
    }

    /// Returns the effective duration if it can be determined.
    ///
    /// For `ScheduledWithEnd` this is `end - start`;
    /// for `ScheduledWithDuration` / `UnspecifiedWithDuration` it is the stored
    /// value.
    #[must_use]
    pub fn duration(&self) -> Option<Duration> {
        match self {
            TimeRange::UnspecifiedWithDuration(d) => Some(*d),
            TimeRange::ScheduledWithDuration { duration, .. } => Some(*duration),
            TimeRange::ScheduledWithEnd { start_time, end_time } => Some(*end_time - *start_time),
            _ => None,
        }
    }

    /// Returns `true` if two scheduled ranges overlap (exclusive endpoint).
    #[must_use]
    pub fn overlaps(&self, other: &TimeRange) -> bool {
        if let (Some(s1), Some(e1), Some(s2), Some(e2)) = (
            self.start_time(),
            self.end_time(),
            other.start_time(),
            other.end_time(),
        ) {
            s1 < e2 && s2 < e1
        } else {
            false
        }
    }

    /// Validate internal consistency.
    ///
    /// Returns `Err` if:
    /// - A scheduled range has start ≥ end.
    /// - A duration or computed duration is ≤ zero.
    pub fn validate(&self) -> Result<(), String> {
        if let (Some(start), Some(end)) = (self.start_time(), self.end_time()) {
            if start >= end {
                return Err("start time must be before end time".to_string());
            }
        }
        if let Some(d) = self.duration() {
            if d <= Duration::zero() {
                return Err("duration must be positive".to_string());
            }
        }
        Ok(())
    }

    /// Check whether a separately provided end time is consistent with this
    /// range.  Used during import when the spreadsheet supplies all three
    /// columns (start, end, duration).
    ///
    /// Returns `Err` with a description when the supplied `end_time` disagrees
    /// with the value computed from this range's start + duration.
    pub fn validate_against_end_time(&self, end_time: NaiveDateTime) -> Result<(), String> {
        if let TimeRange::ScheduledWithDuration { start_time, duration } = self {
            let computed_end = *start_time + *duration;
            if computed_end != end_time {
                return Err(format!(
                    "end time {} does not match start {} + duration {}min (= {})",
                    end_time.format(STORAGE_FMT),
                    start_time.format(STORAGE_FMT),
                    duration.num_minutes(),
                    computed_end.format(STORAGE_FMT),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        parse_datetime(s).unwrap()
    }

    #[test]
    fn parse_datetime_iso() {
        assert_eq!(
            dt("2026-06-26T14:00:00").format("%Y-%m-%d %H:%M").to_string(),
            "2026-06-26 14:00"
        );
    }

    #[test]
    fn parse_datetime_us() {
        assert_eq!(
            dt("6/26/2026 2:00 PM").format("%Y-%m-%d %H:%M").to_string(),
            "2026-06-26 14:00"
        );
    }

    #[test]
    fn parse_datetime_empty_returns_none() {
        assert!(parse_datetime("").is_none());
        assert!(parse_datetime("   ").is_none());
    }

    #[test]
    fn parse_duration_hhmm() {
        assert_eq!(parse_duration("1:30"), Some(Duration::minutes(90)));
        assert_eq!(parse_duration("2:00"), Some(Duration::minutes(120)));
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("90"), Some(Duration::minutes(90)));
    }

    #[test]
    fn parse_duration_empty_returns_none() {
        assert!(parse_duration("").is_none());
    }

    #[test]
    fn time_range_default_is_unspecified() {
        assert_eq!(TimeRange::default(), TimeRange::Unspecified);
    }

    #[test]
    fn time_range_accessors_scheduled_with_duration() {
        let s = dt("2026-06-26T14:00:00");
        let d = Duration::minutes(60);
        let tr = TimeRange::ScheduledWithDuration { start_time: s, duration: d };
        assert!(tr.is_scheduled());
        assert_eq!(tr.start_time(), Some(s));
        assert_eq!(tr.duration(), Some(d));
        assert_eq!(tr.end_time(), Some(dt("2026-06-26T15:00:00")));
    }

    #[test]
    fn time_range_accessors_scheduled_with_end() {
        let s = dt("2026-06-26T14:00:00");
        let e = dt("2026-06-26T15:30:00");
        let tr = TimeRange::ScheduledWithEnd { start_time: s, end_time: e };
        assert!(tr.is_scheduled());
        assert_eq!(tr.start_time(), Some(s));
        assert_eq!(tr.end_time(), Some(e));
        assert_eq!(tr.duration(), Some(Duration::minutes(90)));
    }

    #[test]
    fn time_range_overlaps() {
        let make = |start: &str, end: &str| TimeRange::ScheduledWithEnd {
            start_time: dt(start),
            end_time: dt(end),
        };
        let a = make("2026-06-26T14:00:00", "2026-06-26T15:00:00");
        let b = make("2026-06-26T14:30:00", "2026-06-26T15:30:00");
        let c = make("2026-06-26T16:00:00", "2026-06-26T17:00:00");
        assert!(a.overlaps(&b));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn time_range_serde_round_trip_scheduled_with_duration() {
        let tr = TimeRange::ScheduledWithDuration {
            start_time: dt("2026-06-26T14:00:00"),
            duration: Duration::minutes(60),
        };
        let json = serde_json::to_string(&tr).unwrap();
        let back: TimeRange = serde_json::from_str(&json).unwrap();
        assert_eq!(tr, back);
    }

    #[test]
    fn time_range_serde_round_trip_scheduled_with_end() {
        let tr = TimeRange::ScheduledWithEnd {
            start_time: dt("2026-06-26T14:00:00"),
            end_time: dt("2026-06-26T15:00:00"),
        };
        let json = serde_json::to_string(&tr).unwrap();
        let back: TimeRange = serde_json::from_str(&json).unwrap();
        assert_eq!(tr, back);
    }

    #[test]
    fn time_range_serde_round_trip_unspecified() {
        let json = serde_json::to_string(&TimeRange::Unspecified).unwrap();
        let back: TimeRange = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TimeRange::Unspecified);
    }

    #[test]
    fn validate_inconsistent_end_time() {
        let tr = TimeRange::ScheduledWithDuration {
            start_time: dt("2026-06-26T14:00:00"),
            duration: Duration::minutes(60),
        };
        assert!(tr.validate_against_end_time(dt("2026-06-26T15:00:00")).is_ok());
        assert!(tr.validate_against_end_time(dt("2026-06-26T15:15:00")).is_err());
    }
}
