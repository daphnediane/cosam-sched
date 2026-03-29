/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Time range operations and utilities

use super::TimeRange;
use chrono::{Duration, NaiveDateTime};

impl TimeRange {
    /// Add duration to time range
    pub fn add_duration(&mut self, duration: Duration) {
        match self {
            TimeRange::Unspecified => {
                *self = TimeRange::UnspecifiedWithDuration(duration);
            }
            TimeRange::UnspecifiedWithDuration(_) => {
                *self = TimeRange::UnspecifiedWithDuration(duration);
            }
            TimeRange::UnspecifiedWithEnd(_) => {
                *self = TimeRange::UnspecifiedWithDuration(duration);
            }
            TimeRange::UnspecifiedWithStart(start) => {
                *self = TimeRange::ScheduledWithDuration {
                    start_time: *start,
                    duration,
                };
            }
            TimeRange::ScheduledWithDuration { start_time, .. } => {
                *self = TimeRange::ScheduledWithDuration {
                    start_time: *start_time,
                    duration,
                };
            }
            TimeRange::ScheduledWithEnd { start_time, .. } => {
                *self = TimeRange::ScheduledWithDuration {
                    start_time: *start_time,
                    duration,
                };
            }
        }
    }

    /// Add end time to time range
    pub fn add_end_time(&mut self, end_time: NaiveDateTime) {
        match self {
            TimeRange::Unspecified => {
                *self = TimeRange::UnspecifiedWithEnd(end_time);
            }
            TimeRange::UnspecifiedWithDuration(_) => {
                *self = TimeRange::UnspecifiedWithEnd(end_time);
            }
            TimeRange::UnspecifiedWithEnd(_) => {
                *self = TimeRange::UnspecifiedWithEnd(end_time);
            }
            TimeRange::UnspecifiedWithStart(start) => {
                // Validate: end_time must be after start_time
                if end_time > *start {
                    *self = TimeRange::ScheduledWithEnd {
                        start_time: *start,
                        end_time,
                    };
                } else {
                    // Invalid range: keep as UnspecifiedWithEnd(end_time)
                    *self = TimeRange::UnspecifiedWithEnd(end_time);
                }
            }
            TimeRange::ScheduledWithDuration { start_time, .. } => {
                // Validate: end_time must be after start_time
                if end_time > *start_time {
                    *self = TimeRange::ScheduledWithEnd {
                        start_time: *start_time,
                        end_time,
                    };
                } else {
                    // Invalid range: keep as UnspecifiedWithEnd(end_time)
                    *self = TimeRange::UnspecifiedWithEnd(end_time);
                }
            }
            TimeRange::ScheduledWithEnd { start_time, .. } => {
                // Validate: end_time must be after start_time
                if end_time > *start_time {
                    *self = TimeRange::ScheduledWithEnd {
                        start_time: *start_time,
                        end_time,
                    };
                } else {
                    // Invalid range: keep as UnspecifiedWithEnd(end_time)
                    *self = TimeRange::UnspecifiedWithEnd(end_time);
                }
            }
        }
    }

    /// Add start time to time range
    pub fn add_start_time(&mut self, start_time: NaiveDateTime) {
        match self {
            TimeRange::Unspecified => {
                *self = TimeRange::UnspecifiedWithStart(start_time);
            }
            TimeRange::UnspecifiedWithDuration(duration) => {
                *self = TimeRange::ScheduledWithDuration {
                    start_time,
                    duration: *duration,
                };
            }
            TimeRange::UnspecifiedWithEnd(end) => {
                // Validate: end must be after start_time
                if *end > start_time {
                    *self = TimeRange::ScheduledWithEnd {
                        start_time,
                        end_time: *end,
                    };
                } else {
                    // Invalid range: keep as UnspecifiedWithEnd(end)
                    *self = TimeRange::UnspecifiedWithStart(start_time);
                }
            }
            TimeRange::UnspecifiedWithStart(_) => {
                *self = TimeRange::UnspecifiedWithStart(start_time);
            }
            TimeRange::ScheduledWithDuration { duration, .. } => {
                *self = TimeRange::ScheduledWithDuration {
                    start_time,
                    duration: *duration,
                };
            }
            TimeRange::ScheduledWithEnd { end_time, .. } => {
                // Validate: end_time must be after start_time
                if *end_time > start_time {
                    *self = TimeRange::ScheduledWithEnd {
                        start_time,
                        end_time: *end_time,
                    };
                } else {
                    // Invalid range: keep as UnspecifiedWithEnd(end_time)
                    *self = TimeRange::UnspecifiedWithStart(start_time);
                }
            }
        }
    }

    /// Remove duration from time range
    pub fn remove_duration(&mut self) {
        match self {
            TimeRange::ScheduledWithDuration { start_time, .. } => {
                *self = TimeRange::UnspecifiedWithStart(*start_time);
            }
            TimeRange::UnspecifiedWithDuration(_) => {
                *self = TimeRange::Unspecified;
            }
            TimeRange::ScheduledWithEnd { start_time, .. } => {
                *self = TimeRange::UnspecifiedWithStart(*start_time);
            }
            _ => {} // No duration to remove
        }
    }

    /// Remove end time from time range
    pub fn remove_end_time(&mut self) {
        match self {
            TimeRange::ScheduledWithEnd { start_time, .. } => {
                *self = TimeRange::UnspecifiedWithStart(*start_time);
            }
            TimeRange::UnspecifiedWithEnd(_) => {
                *self = TimeRange::Unspecified;
            }
            TimeRange::ScheduledWithDuration { start_time, .. } => {
                *self = TimeRange::UnspecifiedWithStart(*start_time);
            }
            _ => {} // No end time to remove
        }
    }

    /// Remove start time from time range
    pub fn remove_start_time(&mut self) {
        match self {
            TimeRange::ScheduledWithDuration { duration, .. } => {
                *self = TimeRange::UnspecifiedWithDuration(*duration);
            }
            TimeRange::ScheduledWithEnd { end_time, .. } => {
                *self = TimeRange::UnspecifiedWithEnd(*end_time);
            }
            TimeRange::UnspecifiedWithStart(_) => {
                *self = TimeRange::Unspecified;
            }
            _ => {} // No start time to remove
        }
    }

    /// Validate time range consistency
    pub fn validate(&self) -> Result<(), String> {
        if let (Some(start), Some(end)) = (self.start_time(), self.end_time()) {
            if start >= end {
                return Err("Start time must be before end time".to_string());
            }
        }

        if let Some(duration) = self.duration() {
            if duration <= Duration::zero() {
                return Err("Duration must be positive".to_string());
            }
        }

        Ok(())
    }
}
