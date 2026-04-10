/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Mutation methods for [`TimeRange`].
//!
//! Each method follows the invariant: **setting end time or duration never
//! adjusts start time**.  Setting start time preserves whichever end/duration
//! was active, unless the new start would violate ordering.

use super::TimeRange;
use chrono::{Duration, NaiveDateTime};

impl TimeRange {
    // --- Add (set) ----------------------------------------------------------

    /// Set the start time, transitioning to the appropriate state.
    ///
    /// - If a duration was already stored, produces `ScheduledWithDuration`.
    /// - If an end time was already stored and `start < end`, produces
    ///   `ScheduledWithEnd`; otherwise falls back to `UnspecifiedWithStart`.
    pub fn add_start_time(&mut self, start_time: NaiveDateTime) {
        *self = match self {
            TimeRange::Unspecified | TimeRange::UnspecifiedWithStart(_) => {
                TimeRange::UnspecifiedWithStart(start_time)
            }
            TimeRange::UnspecifiedWithDuration(d) => TimeRange::ScheduledWithDuration {
                start_time,
                duration: *d,
            },
            TimeRange::UnspecifiedWithEnd(e) => {
                if start_time < *e {
                    TimeRange::ScheduledWithEnd { start_time, end_time: *e }
                } else {
                    TimeRange::UnspecifiedWithStart(start_time)
                }
            }
            TimeRange::ScheduledWithDuration { duration, .. } => {
                TimeRange::ScheduledWithDuration { start_time, duration: *duration }
            }
            TimeRange::ScheduledWithEnd { end_time, .. } => {
                if start_time < *end_time {
                    TimeRange::ScheduledWithEnd { start_time, end_time: *end_time }
                } else {
                    TimeRange::UnspecifiedWithStart(start_time)
                }
            }
        };
    }

    /// Set the end time, transitioning to the appropriate state.
    ///
    /// Start time is **never** adjusted.  If a start time is known and
    /// `end > start`, produces `ScheduledWithEnd`; otherwise stores as
    /// `UnspecifiedWithEnd`.
    pub fn add_end_time(&mut self, end_time: NaiveDateTime) {
        *self = match self {
            TimeRange::Unspecified
            | TimeRange::UnspecifiedWithDuration(_)
            | TimeRange::UnspecifiedWithEnd(_) => TimeRange::UnspecifiedWithEnd(end_time),
            TimeRange::UnspecifiedWithStart(s) => {
                if end_time > *s {
                    TimeRange::ScheduledWithEnd { start_time: *s, end_time }
                } else {
                    TimeRange::UnspecifiedWithEnd(end_time)
                }
            }
            TimeRange::ScheduledWithDuration { start_time, .. }
            | TimeRange::ScheduledWithEnd { start_time, .. } => {
                if end_time > *start_time {
                    TimeRange::ScheduledWithEnd { start_time: *start_time, end_time }
                } else {
                    TimeRange::UnspecifiedWithEnd(end_time)
                }
            }
        };
    }

    /// Set the duration, transitioning to the appropriate state.
    ///
    /// Start time is **never** adjusted.  If a start time is known, produces
    /// `ScheduledWithDuration`; otherwise stores as `UnspecifiedWithDuration`.
    pub fn add_duration(&mut self, duration: Duration) {
        *self = match self {
            TimeRange::Unspecified
            | TimeRange::UnspecifiedWithDuration(_)
            | TimeRange::UnspecifiedWithEnd(_) => TimeRange::UnspecifiedWithDuration(duration),
            TimeRange::UnspecifiedWithStart(s) => {
                TimeRange::ScheduledWithDuration { start_time: *s, duration }
            }
            TimeRange::ScheduledWithDuration { start_time, .. }
            | TimeRange::ScheduledWithEnd { start_time, .. } => {
                TimeRange::ScheduledWithDuration { start_time: *start_time, duration }
            }
        };
    }

    // --- Remove (clear) -----------------------------------------------------

    /// Remove the start time.
    ///
    /// Any stored duration or end time is preserved in an `Unspecified*`
    /// variant.
    pub fn remove_start_time(&mut self) {
        *self = match self {
            TimeRange::UnspecifiedWithStart(_) => TimeRange::Unspecified,
            TimeRange::ScheduledWithDuration { duration, .. } => {
                TimeRange::UnspecifiedWithDuration(*duration)
            }
            TimeRange::ScheduledWithEnd { end_time, .. } => {
                TimeRange::UnspecifiedWithEnd(*end_time)
            }
            _ => return,
        };
    }

    /// Remove the end time.
    ///
    /// For `ScheduledWithEnd` the stored end time is dropped and only the
    /// start time is retained.  For `ScheduledWithDuration` the computed end
    /// is removed by dropping the duration as well (since end was derived from
    /// it), leaving `UnspecifiedWithStart`.
    pub fn remove_end_time(&mut self) {
        *self = match self {
            TimeRange::UnspecifiedWithEnd(_) => TimeRange::Unspecified,
            TimeRange::ScheduledWithEnd { start_time, .. } => {
                TimeRange::UnspecifiedWithStart(*start_time)
            }
            TimeRange::ScheduledWithDuration { start_time, .. } => {
                TimeRange::UnspecifiedWithStart(*start_time)
            }
            _ => return,
        };
    }

    /// Remove the duration.
    ///
    /// For `ScheduledWithDuration` the duration is dropped and only the start
    /// time is retained.  For `ScheduledWithEnd` the computed duration is
    /// removed by dropping the end time as well, leaving `UnspecifiedWithStart`.
    pub fn remove_duration(&mut self) {
        *self = match self {
            TimeRange::UnspecifiedWithDuration(_) => TimeRange::Unspecified,
            TimeRange::ScheduledWithDuration { start_time, .. } => {
                TimeRange::UnspecifiedWithStart(*start_time)
            }
            TimeRange::ScheduledWithEnd { start_time, .. } => {
                TimeRange::UnspecifiedWithStart(*start_time)
            }
            _ => return,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::parse_datetime;

    fn dt(s: &str) -> NaiveDateTime {
        parse_datetime(s).unwrap()
    }

    // --- add_start_time -----------------------------------------------------

    #[test]
    fn add_start_to_unspecified() {
        let mut tr = TimeRange::Unspecified;
        tr.add_start_time(dt("2026-06-26T14:00:00"));
        assert_eq!(tr, TimeRange::UnspecifiedWithStart(dt("2026-06-26T14:00:00")));
    }

    #[test]
    fn add_start_to_unspecified_with_duration() {
        let mut tr = TimeRange::UnspecifiedWithDuration(Duration::minutes(60));
        tr.add_start_time(dt("2026-06-26T14:00:00"));
        assert_eq!(
            tr,
            TimeRange::ScheduledWithDuration {
                start_time: dt("2026-06-26T14:00:00"),
                duration: Duration::minutes(60)
            }
        );
    }

    #[test]
    fn add_start_to_unspecified_with_end_valid() {
        let mut tr = TimeRange::UnspecifiedWithEnd(dt("2026-06-26T15:00:00"));
        tr.add_start_time(dt("2026-06-26T14:00:00"));
        assert_eq!(
            tr,
            TimeRange::ScheduledWithEnd {
                start_time: dt("2026-06-26T14:00:00"),
                end_time: dt("2026-06-26T15:00:00")
            }
        );
    }

    #[test]
    fn add_start_to_unspecified_with_end_invalid_drops_end() {
        let mut tr = TimeRange::UnspecifiedWithEnd(dt("2026-06-26T13:00:00"));
        tr.add_start_time(dt("2026-06-26T14:00:00"));
        assert_eq!(tr, TimeRange::UnspecifiedWithStart(dt("2026-06-26T14:00:00")));
    }

    #[test]
    fn add_start_to_scheduled_with_duration_keeps_duration() {
        let mut tr = TimeRange::ScheduledWithDuration {
            start_time: dt("2026-06-26T10:00:00"),
            duration: Duration::minutes(90),
        };
        tr.add_start_time(dt("2026-06-26T14:00:00"));
        assert_eq!(
            tr,
            TimeRange::ScheduledWithDuration {
                start_time: dt("2026-06-26T14:00:00"),
                duration: Duration::minutes(90)
            }
        );
    }

    #[test]
    fn add_start_to_scheduled_with_end_keeps_end_when_valid() {
        let mut tr = TimeRange::ScheduledWithEnd {
            start_time: dt("2026-06-26T10:00:00"),
            end_time: dt("2026-06-26T15:00:00"),
        };
        tr.add_start_time(dt("2026-06-26T14:00:00"));
        assert_eq!(
            tr,
            TimeRange::ScheduledWithEnd {
                start_time: dt("2026-06-26T14:00:00"),
                end_time: dt("2026-06-26T15:00:00")
            }
        );
    }

    #[test]
    fn add_start_to_scheduled_with_end_drops_end_when_invalid() {
        let mut tr = TimeRange::ScheduledWithEnd {
            start_time: dt("2026-06-26T10:00:00"),
            end_time: dt("2026-06-26T15:00:00"),
        };
        tr.add_start_time(dt("2026-06-26T16:00:00"));
        assert_eq!(tr, TimeRange::UnspecifiedWithStart(dt("2026-06-26T16:00:00")));
    }

    // --- add_end_time -------------------------------------------------------

    #[test]
    fn add_end_never_adjusts_start() {
        let mut tr = TimeRange::ScheduledWithDuration {
            start_time: dt("2026-06-26T14:00:00"),
            duration: Duration::minutes(60),
        };
        tr.add_end_time(dt("2026-06-26T16:00:00"));
        assert_eq!(tr.start_time(), Some(dt("2026-06-26T14:00:00")));
        assert_eq!(tr.end_time(), Some(dt("2026-06-26T16:00:00")));
    }

    #[test]
    fn add_end_invalid_stores_as_unspecified_with_end() {
        let mut tr = TimeRange::ScheduledWithDuration {
            start_time: dt("2026-06-26T14:00:00"),
            duration: Duration::minutes(60),
        };
        tr.add_end_time(dt("2026-06-26T13:00:00"));
        assert_eq!(tr, TimeRange::UnspecifiedWithEnd(dt("2026-06-26T13:00:00")));
    }

    // --- add_duration -------------------------------------------------------

    #[test]
    fn add_duration_never_adjusts_start() {
        let mut tr = TimeRange::ScheduledWithEnd {
            start_time: dt("2026-06-26T14:00:00"),
            end_time: dt("2026-06-26T15:00:00"),
        };
        tr.add_duration(Duration::minutes(90));
        assert_eq!(tr.start_time(), Some(dt("2026-06-26T14:00:00")));
        assert_eq!(tr.duration(), Some(Duration::minutes(90)));
    }

    // --- remove methods -----------------------------------------------------

    #[test]
    fn remove_start_from_scheduled_with_duration() {
        let mut tr = TimeRange::ScheduledWithDuration {
            start_time: dt("2026-06-26T14:00:00"),
            duration: Duration::minutes(60),
        };
        tr.remove_start_time();
        assert_eq!(tr, TimeRange::UnspecifiedWithDuration(Duration::minutes(60)));
    }

    #[test]
    fn remove_start_from_scheduled_with_end() {
        let mut tr = TimeRange::ScheduledWithEnd {
            start_time: dt("2026-06-26T14:00:00"),
            end_time: dt("2026-06-26T15:00:00"),
        };
        tr.remove_start_time();
        assert_eq!(tr, TimeRange::UnspecifiedWithEnd(dt("2026-06-26T15:00:00")));
    }

    #[test]
    fn remove_end_from_scheduled_with_end() {
        let mut tr = TimeRange::ScheduledWithEnd {
            start_time: dt("2026-06-26T14:00:00"),
            end_time: dt("2026-06-26T15:00:00"),
        };
        tr.remove_end_time();
        assert_eq!(tr, TimeRange::UnspecifiedWithStart(dt("2026-06-26T14:00:00")));
    }

    #[test]
    fn remove_duration_from_scheduled_with_duration() {
        let mut tr = TimeRange::ScheduledWithDuration {
            start_time: dt("2026-06-26T14:00:00"),
            duration: Duration::minutes(60),
        };
        tr.remove_duration();
        assert_eq!(tr, TimeRange::UnspecifiedWithStart(dt("2026-06-26T14:00:00")));
    }
}
