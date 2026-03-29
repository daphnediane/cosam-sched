/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Time handling for TimeRange

pub mod range;

use chrono::{Duration, NaiveDateTime};
use std::fmt;

// range.rs adds methods to TimeRange via impl blocks

/// Time range state following formal state transitions
#[derive(Debug, Clone, PartialEq)]
pub enum TimeRange {
    /// No timing information specified
    Unspecified,
    /// Unscheduled but has duration specified
    UnspecifiedWithDuration(Duration),
    /// Unscheduled but has end specified
    UnspecifiedWithEnd(NaiveDateTime),
    /// Unscheduled but has start time specified (no duration or end time yet)
    UnspecifiedWithStart(NaiveDateTime),
    /// Fully scheduled with start time + duration
    ScheduledWithDuration {
        start_time: NaiveDateTime,
        duration: Duration,
    },
    /// Fully scheduled with start time + end time
    ScheduledWithEnd {
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    },
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::Unspecified
    }
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeRange::Unspecified => write!(f, "Unspecified"),
            TimeRange::UnspecifiedWithDuration(duration) => {
                write!(f, "Unspecified ({} min)", duration.num_minutes())
            }
            TimeRange::UnspecifiedWithEnd(end) => {
                write!(f, "Unspecified (ends at {})", end.format("%Y-%m-%d %H:%M"))
            }
            TimeRange::UnspecifiedWithStart(start) => {
                write!(
                    f,
                    "Unspecified (starts at {})",
                    start.format("%Y-%m-%d %H:%M")
                )
            }
            TimeRange::ScheduledWithDuration {
                start_time,
                duration,
            } => {
                write!(
                    f,
                    "{} ({} min)",
                    start_time.format("%Y-%m-%d %H:%M"),
                    duration.num_minutes()
                )
            }
            TimeRange::ScheduledWithEnd {
                start_time,
                end_time,
            } => {
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

impl TimeRange {
    /// Check if this time range is scheduled (has a start time)
    pub fn is_scheduled(&self) -> bool {
        matches!(
            self,
            TimeRange::ScheduledWithDuration { .. } | TimeRange::ScheduledWithEnd { .. }
        )
    }

    /// Get the start time if available
    pub fn start_time(&self) -> Option<NaiveDateTime> {
        match self {
            TimeRange::UnspecifiedWithStart(start) => Some(*start),
            TimeRange::ScheduledWithDuration { start_time, .. } => Some(*start_time),
            TimeRange::ScheduledWithEnd { start_time, .. } => Some(*start_time),
            _ => None,
        }
    }

    /// Get the end time if available
    pub fn end_time(&self) -> Option<NaiveDateTime> {
        match self {
            TimeRange::UnspecifiedWithEnd(end) => Some(*end),
            TimeRange::ScheduledWithEnd { end_time, .. } => Some(*end_time),
            TimeRange::ScheduledWithDuration {
                start_time,
                duration,
            } => Some(*start_time + *duration),
            _ => None,
        }
    }

    /// Get the duration if available
    pub fn duration(&self) -> Option<Duration> {
        match self {
            TimeRange::UnspecifiedWithDuration(duration) => Some(*duration),
            TimeRange::ScheduledWithDuration { duration, .. } => Some(*duration),
            TimeRange::ScheduledWithEnd {
                start_time,
                end_time,
            } => Some(*end_time - *start_time),
            _ => None,
        }
    }

    /// Check if the time range overlaps with another
    pub fn overlaps(&self, other: &TimeRange) -> bool {
        if let (Some(self_start), Some(self_end)) = (self.start_time(), self.end_time()) {
            if let (Some(other_start), Some(other_end)) = (other.start_time(), other.end_time()) {
                return self_start < other_end && other_start < self_end;
            }
        }
        false
    }
}
