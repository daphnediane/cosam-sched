/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Timezone resolution and iCalendar `VTIMEZONE` generation.
//!
//! Schedule timestamps are stored as naive wall-clock values; the schedule's
//! [`crate::schedule::ScheduleMetadata::timezone`] names the IANA zone they are
//! expressed in.  This module turns user-supplied zone names (including common
//! abbreviations) into a [`chrono_tz::Tz`], resolves a default zone when none is
//! given, and builds the `VTIMEZONE` component the widget embeds into exported
//! `.ics` files so calendar apps anchor events correctly.

use chrono::{Duration, NaiveDate, NaiveDateTime, TimeZone};
use chrono_tz::{OffsetComponents, OffsetName, Tz};
use std::str::FromStr;

/// Parse a timezone name into a [`Tz`], accepting names liberally.
///
/// Resolution order:
/// 1. [`Tz::from_str`] directly — handles full IANA names
///    (`America/New_York`), the tz-database POSIX zones (`EST5EDT`, `EST`,
///    `MST7MDT`, `CST6CDT`, `PST8PDT`, `HST`, `UTC`, `GMT`) and `Etc/GMT±N`.
/// 2. A small case-insensitive alias table for bare daylight abbreviations
///    that are *not* tz-database zones (`EDT`, `CDT`, `MDT`, `PDT`).
///
/// Note that bare standard abbreviations (`EST`, `CST`, `MST`, `PST`) resolve to
/// chrono-tz's *fixed-offset* zones (no DST); use the DST-aware POSIX form
/// (`EST5EDT`) or a full IANA name when daylight handling matters.
#[must_use]
pub fn parse_tz(name: &str) -> Option<Tz> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(tz) = Tz::from_str(trimmed) {
        return Some(tz);
    }
    let canonical = match trimmed.to_ascii_uppercase().as_str() {
        "EDT" => "EST5EDT",
        "CDT" => "CST6CDT",
        "MDT" => "MST7MDT",
        "PDT" => "PST8PDT",
        _ => return None,
    };
    Tz::from_str(canonical).ok()
}

/// The user's local IANA timezone name, if the platform can report one.
#[must_use]
pub fn local_tz_name() -> Option<String> {
    iana_time_zone::get_timezone().ok()
}

/// Resolve the effective timezone name from the available sources, in priority
/// order, falling back to `"UTC"` if nothing else validates.
///
/// Each candidate is validated with [`parse_tz`]; the *original* name is
/// returned (so a chosen abbreviation like `EDT` is normalized to its canonical
/// zone) and is guaranteed to round-trip through [`parse_tz`].
#[must_use]
pub fn resolve_timezone(candidates: &[Option<&str>]) -> String {
    for cand in candidates.iter().flatten() {
        if let Some(tz) = parse_tz(cand) {
            return tz.name().to_string();
        }
    }
    if let Some(local) = local_tz_name() {
        if parse_tz(&local).is_some() {
            return local;
        }
    }
    "UTC".to_string()
}

/// Format a UTC offset (in seconds) as the iCalendar `±HHMM[SS]` form.
fn fmt_offset(total_seconds: i64) -> String {
    let sign = if total_seconds < 0 { '-' } else { '+' };
    let abs = total_seconds.abs();
    let h = abs / 3600;
    let m = (abs % 3600) / 60;
    let s = abs % 60;
    if s == 0 {
        format!("{sign}{h:02}{m:02}")
    } else {
        format!("{sign}{h:02}{m:02}{s:02}")
    }
}

/// One observed UTC-offset segment within the scanned window.
struct Segment {
    /// Local wall time at which this segment begins, in terms of the *previous*
    /// offset (RFC 5545 `DTSTART`).
    onset_local: NaiveDateTime,
    offset_from: i64,
    offset_to: i64,
    is_dst: bool,
    name: String,
}

/// Build an iCalendar `VTIMEZONE` component for `tz` covering `[start, end]`.
///
/// The component lists the offset in effect at the window start plus every
/// offset transition within the window as explicit (non-recurring)
/// STANDARD/DAYLIGHT sub-components.  Returns an empty string for zones with no
/// DST and a zero base offset (UTC/GMT), where a `VTIMEZONE` adds nothing.
///
/// The returned text has no trailing newline; callers join it with CRLF.
#[must_use]
pub fn build_vtimezone(tz: Tz, start: NaiveDateTime, end: NaiveDateTime) -> String {
    // UTC needs no VTIMEZONE — events are already absolute.
    if tz == chrono_tz::UTC {
        return String::new();
    }

    // Scan from a day before the window to a day after, so an onset right at a
    // boundary is captured.  Work in UTC instants.
    let scan_start = start - Duration::days(1);
    let scan_end = end + Duration::days(1);

    let offset_at = |utc: NaiveDateTime| -> (i64, i64, String) {
        let off = tz.offset_from_utc_datetime(&utc);
        let total = off.base_utc_offset() + off.dst_offset();
        let dst = off.dst_offset();
        (
            total.num_seconds(),
            dst.num_seconds(),
            off.abbreviation().unwrap_or("").to_string(),
        )
    };

    let mut segments: Vec<Segment> = Vec::new();
    let (mut prev_total, prev_dst, prev_name) = offset_at(scan_start);
    // Seed with the offset in effect at the window start.
    segments.push(Segment {
        onset_local: scan_start + Duration::seconds(prev_total),
        offset_from: prev_total,
        offset_to: prev_total,
        is_dst: prev_dst != 0,
        name: prev_name,
    });

    // Step hour by hour, refining each detected change to the minute.
    let mut cursor = scan_start;
    while cursor < scan_end {
        let next = cursor + Duration::hours(1);
        let (next_total, _, _) = offset_at(next);
        if next_total != prev_total {
            // Binary-search the transition instant to the minute within (cursor, next].
            let mut lo = cursor;
            let mut hi = next;
            while hi - lo > Duration::minutes(1) {
                let mid = lo + (hi - lo) / 2;
                let (mid_total, _, _) = offset_at(mid);
                if mid_total == prev_total {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let (to_total, to_dst, to_name) = offset_at(hi);
            segments.push(Segment {
                // DTSTART is the wall time of the onset using the FROM offset.
                onset_local: hi + Duration::seconds(prev_total),
                offset_from: prev_total,
                offset_to: to_total,
                is_dst: to_dst != 0,
                name: to_name,
            });
            prev_total = to_total;
        }
        cursor = next;
    }

    let mut out = String::new();
    out.push_str("BEGIN:VTIMEZONE\r\n");
    out.push_str(&format!("TZID:{}\r\n", tz.name()));
    for seg in &segments {
        let kind = if seg.is_dst { "DAYLIGHT" } else { "STANDARD" };
        out.push_str(&format!("BEGIN:{kind}\r\n"));
        out.push_str(&format!(
            "DTSTART:{}\r\n",
            seg.onset_local.format("%Y%m%dT%H%M%S")
        ));
        out.push_str(&format!("TZOFFSETFROM:{}\r\n", fmt_offset(seg.offset_from)));
        out.push_str(&format!("TZOFFSETTO:{}\r\n", fmt_offset(seg.offset_to)));
        if !seg.name.is_empty() {
            out.push_str(&format!("TZNAME:{}\r\n", seg.name));
        }
        out.push_str(&format!("END:{kind}\r\n"));
    }
    out.push_str("END:VTIMEZONE");
    out
}

/// Helper: a `NaiveDateTime` at midnight on the given Y/M/D, for callers that
/// only have a date bound.
#[must_use]
pub fn date_midnight(date: NaiveDate) -> NaiveDateTime {
    date.and_hms_opt(0, 0, 0).expect("midnight is always valid")
}

/// Format a calendar day's weekday label, disambiguated against the schedule's
/// date range (FEATURE-154). Mirrors the print layout's day headings so the
/// precomputed widget day timelines read identically:
///
/// - within one ISO week: just the weekday (`"Friday"`)
/// - same month, multiple weeks: weekday + day-of-month (`"Friday 5"`)
/// - spanning months: weekday + month + day (`"Friday Jun 5"`)
#[must_use]
pub fn day_label(date: NaiveDate, min_date: NaiveDate, max_date: NaiveDate) -> String {
    use chrono::Datelike;
    let weekday = date.format("%A").to_string();
    let same_week = min_date.iso_week() == max_date.iso_week();
    let same_month = min_date.year() == max_date.year() && min_date.month() == max_date.month();
    if same_week {
        weekday
    } else if same_month {
        format!("{} {}", weekday, date.day())
    } else {
        format!("{} {} {}", weekday, date.format("%b"), date.day())
    }
}

/// Convert Unix epoch seconds to a naive wall-clock datetime expressed in the
/// named IANA timezone (FEATURE-154). An empty or unrecognized zone is treated
/// as UTC, mirroring the export-side interpretation. This is the inverse of the
/// naive-wall-clock → epoch conversion performed during widget export.
#[must_use]
pub fn epoch_to_local(epoch: i64, tz_name: &str) -> NaiveDateTime {
    let tz = parse_tz(tz_name).unwrap_or(Tz::UTC);
    tz.timestamp_opt(epoch, 0)
        .single()
        .map(|dt| dt.naive_local())
        .unwrap_or_else(|| {
            chrono::DateTime::from_timestamp(epoch, 0)
                .unwrap_or_default()
                .naive_utc()
        })
}

/// Format epoch seconds as the naive wall-clock ISO 8601 string
/// (`%Y-%m-%dT%H:%M:%S`) in the named timezone — the same shape the widget
/// formats previously carried, for consumers that still operate on strings.
#[must_use]
pub fn epoch_to_local_iso(epoch: i64, tz_name: &str) -> String {
    epoch_to_local(epoch, tz_name)
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, m: u32, d: u32, h: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, 0, 0)
            .unwrap()
    }

    #[test]
    fn parse_full_iana() {
        assert_eq!(parse_tz("America/New_York"), Some(Tz::America__New_York));
    }

    #[test]
    fn parse_posix_zone() {
        assert_eq!(parse_tz("EST5EDT"), Some(Tz::EST5EDT));
        assert_eq!(parse_tz("UTC"), Some(Tz::UTC));
    }

    #[test]
    fn parse_daylight_abbrev_alias() {
        assert_eq!(parse_tz("EDT"), Some(Tz::EST5EDT));
        assert_eq!(parse_tz("edt"), Some(Tz::EST5EDT));
        assert_eq!(parse_tz("PDT"), Some(Tz::PST8PDT));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(parse_tz(""), None);
        assert_eq!(parse_tz("Not/AZone"), None);
    }

    #[test]
    fn resolve_prefers_first_valid() {
        assert_eq!(
            resolve_timezone(&[None, Some("nonsense"), Some("EDT"), Some("UTC")]),
            "EST5EDT"
        );
    }

    #[test]
    fn vtimezone_empty_for_utc() {
        assert_eq!(
            build_vtimezone(Tz::UTC, dt(2026, 6, 26, 9), dt(2026, 6, 28, 18)),
            ""
        );
    }

    #[test]
    fn vtimezone_summer_window_is_daylight() {
        // A late-June weekend in New York is fully within EDT (UTC-4), no
        // transition — one DAYLIGHT segment.
        let vt = build_vtimezone(
            Tz::America__New_York,
            dt(2026, 6, 26, 9),
            dt(2026, 6, 28, 18),
        );
        assert!(vt.contains("BEGIN:VTIMEZONE"));
        assert!(vt.contains("TZID:America/New_York"));
        assert!(vt.contains("TZOFFSETTO:-0400"));
        assert!(vt.contains("BEGIN:DAYLIGHT"));
    }

    #[test]
    fn vtimezone_captures_spring_forward() {
        // US DST began 2026-03-08 02:00 local. A window spanning it must show a
        // STANDARD→DAYLIGHT transition (offset -0500 → -0400).
        let vt = build_vtimezone(Tz::America__New_York, dt(2026, 3, 7, 0), dt(2026, 3, 9, 0));
        assert!(vt.contains("TZOFFSETFROM:-0500"));
        assert!(vt.contains("TZOFFSETTO:-0400"));
        assert!(vt.contains("BEGIN:DAYLIGHT"));
    }
}
