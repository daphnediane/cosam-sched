/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use chrono::NaiveDateTime;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::source_info::{ChangeState, SourceInfo};
use super::time::{
    TimeRange, deserialize_optional_datetime, deserialize_optional_duration,
    serialize_optional_datetime, serialize_optional_duration,
};
use crate::data::time;

/// Input types for flexible start time setter
pub enum StartTimeInput {
    String(String),
    DateTime(chrono::NaiveDateTime),
}

impl From<String> for StartTimeInput {
    fn from(s: String) -> Self {
        StartTimeInput::String(s)
    }
}

impl From<&str> for StartTimeInput {
    fn from(s: &str) -> Self {
        StartTimeInput::String(s.to_string())
    }
}

impl From<chrono::NaiveDateTime> for StartTimeInput {
    fn from(dt: chrono::NaiveDateTime) -> Self {
        StartTimeInput::DateTime(dt)
    }
}

/// Input types for flexible end time setter
pub enum EndTimeInput {
    String(String),
    DateTime(chrono::NaiveDateTime),
}

impl From<String> for EndTimeInput {
    fn from(s: String) -> Self {
        EndTimeInput::String(s)
    }
}

impl From<&str> for EndTimeInput {
    fn from(s: &str) -> Self {
        EndTimeInput::String(s.to_string())
    }
}

impl From<chrono::NaiveDateTime> for EndTimeInput {
    fn from(dt: chrono::NaiveDateTime) -> Self {
        EndTimeInput::DateTime(dt)
    }
}

/// Input types for flexible duration setter
pub enum DurationInput {
    Minutes(u32),
    Duration(chrono::Duration),
}

impl From<u32> for DurationInput {
    fn from(minutes: u32) -> Self {
        DurationInput::Minutes(minutes)
    }
}

impl From<chrono::Duration> for DurationInput {
    fn from(duration: chrono::Duration) -> Self {
        DurationInput::Duration(duration)
    }
}

/// Represents extra fields from non-standard spreadsheet columns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtraValue {
    String(String),
    Formula(FormulaValue),
}

/// Represents a formula with its evaluated value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormulaValue {
    pub formula: String,
    pub value: String,
}

/// Additional non-standard spreadsheet columns
pub type ExtraFields = IndexMap<String, ExtraValue>;

/// A fully self-contained panel entry in the flat model.
///
/// Each panel belongs to a [`super::panel_set::PanelSet`] identified by
/// `base_id`.  A panel may carry optional `part_num` / `session_num` to
/// reflect XLSX part/session numbering, but those are informational only —
/// the combination (`base_id`, `part_num`, `session_num`) forms a logical
/// key while `id` is the canonical unique identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    /// Full unique identifier (e.g. `"GP002P1S2"`).
    pub id: String,
    /// Base ID of the containing [`super::panel_set::PanelSet`] (e.g. `"GP002"`).
    pub base_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_num: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_num: Option<u32>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_panelist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_reg_max: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simple_tix_event: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub have_ticket_image: Option<bool>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_free: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_kids: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_full: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_panelist: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sewing_machines: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_ids: Vec<u32>,
    #[serde(default)]
    pub timing: TimeRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats_sold: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credited_presenters: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uncredited_presenters: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes_non_printing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workshop_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_needs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub av_notes: Option<String>,
    #[serde(skip)]
    pub source: Option<SourceInfo>,
    #[serde(skip)]
    pub change_state: ChangeState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<super::event::EventConflict>,
    #[serde(default, alias = "extras", skip_serializing_if = "IndexMap::is_empty")]
    pub metadata: ExtraFields,
}

impl Panel {
    /// Create a new empty panel.
    pub fn new(id: impl Into<String>, base_id: impl Into<String>) -> Self {
        Panel {
            id: id.into(),
            base_id: base_id.into(),
            part_num: None,
            session_num: None,
            name: String::new(),
            panel_type: None,
            description: None,
            note: None,
            prereq: None,
            alt_panelist: None,
            cost: None,
            capacity: None,
            pre_reg_max: None,
            difficulty: None,
            ticket_url: None,
            simple_tix_event: None,
            have_ticket_image: None,
            is_free: false,
            is_kids: false,
            is_full: false,
            hide_panelist: false,
            sewing_machines: false,
            room_ids: Vec::new(),
            timing: TimeRange::Unspecified,
            seats_sold: None,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            notes_non_printing: None,
            workshop_notes: None,
            power_needs: None,
            av_notes: None,
            source: None,
            change_state: ChangeState::Unchanged,
            conflicts: Vec::new(),
            metadata: IndexMap::new(),
        }
    }

    /// Returns the effective duration as a chrono::Duration, or None if the panel is unscheduled
    /// or has invalid duration information.
    /// Returns the effective duration of this panel.
    /// Returns None if:
    /// - No timing information is set
    /// - Duration is zero or negative
    /// - Inconsistent timing state
    pub fn effective_duration(&self) -> Option<chrono::Duration> {
        let duration = self.timing.duration();
        duration.filter(|&d| d > chrono::Duration::zero())
    }

    /// Returns the effective end time, or None if it cannot be determined.
    /// Returns None if:
    /// - No timing information is set
    /// - Cannot calculate end time from available information
    pub fn effective_end_time(&self) -> Option<chrono::NaiveDateTime> {
        self.timing.effective_end_time()
    }

    /// Returns the effective duration in minutes, or None if the panel is unscheduled.
    /// This is a convenience method that converts the chrono::Duration to minutes.
    pub fn effective_duration_minutes(&self) -> Option<u32> {
        self.effective_duration().map(|d| d.num_minutes() as u32)
    }

    /// Returns the start time as a formatted string, or None if not set.
    /// This is a convenience method for user-facing code that needs string representation.
    pub fn start_time_str(&self) -> Option<String> {
        self.timing.start_time_str()
    }

    /// Returns the end time as a formatted string, or None if not set.
    /// This is a convenience method for user-facing code that needs string representation.
    pub fn end_time_str(&self) -> Option<String> {
        self.timing.end_time_str()
    }

    /// Returns the effective end time as a formatted string, or None if not set.
    /// This is a convenience method for user-facing code that needs string representation.
    pub fn effective_end_time_str(&self) -> Option<String> {
        self.effective_end_time().map(|dt| time::format_storage(dt))
    }

    /// Sets the start time from a string using the storage format.
    /// Returns true if parsing succeeded, false otherwise.
    pub fn set_start_time_from_str(&mut self, time_str: &str) -> bool {
        self.timing.set_start_time_from_str(time_str)
    }

    /// Sets the end time from a string using the storage format.
    /// Returns true if parsing succeeded, false otherwise.
    pub fn set_end_time_from_str(&mut self, time_str: &str) -> bool {
        self.timing.set_end_time_from_str(time_str)
    }

    /// Sets the duration from minutes.
    pub fn set_duration_minutes(&mut self, minutes: u32) {
        self.timing
            .set_duration(chrono::Duration::minutes(minutes as i64));
    }

    /// Flexible setter for start time - accepts either string or NaiveDateTime
    pub fn set_start_time_flexible<T>(&mut self, time_input: T) -> Result<(), String>
    where
        T: Into<StartTimeInput>,
    {
        match time_input.into() {
            StartTimeInput::String(s) => {
                if self.timing.set_start_time_from_str(&s) {
                    Ok(())
                } else {
                    Err(format!("Invalid datetime format: {}", s))
                }
            }
            StartTimeInput::DateTime(dt) => {
                self.timing.set_start_time(dt);
                Ok(())
            }
        }
    }

    /// Flexible setter for end time - accepts either string or NaiveDateTime
    pub fn set_end_time_flexible<T>(&mut self, time_input: T) -> Result<(), String>
    where
        T: Into<EndTimeInput>,
    {
        match time_input.into() {
            EndTimeInput::String(s) => {
                if let Some(dt) = time::parse_datetime(&s) {
                    self.timing.set_end_time(dt);
                    Ok(())
                } else {
                    Err(format!("Invalid datetime format: {}", s))
                }
            }
            EndTimeInput::DateTime(dt) => {
                self.timing.set_end_time(dt);
                Ok(())
            }
        }
    }

    /// Flexible setter for duration - accepts either minutes (u32) or chrono::Duration
    pub fn set_duration_flexible<T>(&mut self, duration_input: T)
    where
        T: Into<DurationInput>,
    {
        match duration_input.into() {
            DurationInput::Minutes(minutes) => {
                self.timing
                    .set_duration(chrono::Duration::minutes(minutes as i64));
            }
            DurationInput::Duration(duration) => {
                self.timing.set_duration(duration);
            }
        }
    }

    /// Returns `true` if this panel has scheduling information (time + room + duration/end).
    /// A panel is considered unscheduled if it:
    /// - Has no name
    /// - Has no room
    /// - Has no start time
    /// - Does not have a valid positive duration
    pub fn is_scheduled(&self) -> bool {
        // Check for empty name or rooms
        if self.name.trim().is_empty() || self.room_ids.is_empty() {
            return false;
        }

        // Use TimeRange's is_scheduled which accounts for invalid durations
        self.timing.is_scheduled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_scheduled_valid_panel() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "Test Panel".to_string();
        panel.timing = TimeRange::Scheduled {
            start_time: chrono::NaiveDateTime::parse_from_str(
                "2026-06-26T12:00:00",
                "%Y-%m-%dT%H:%M:%S",
            )
            .unwrap(),
            duration: chrono::Duration::minutes(60),
        };
        panel.room_ids = vec![1];

        assert!(panel.is_scheduled());
    }

    #[test]
    fn test_is_scheduled_no_name() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "".to_string();
        panel.timing = TimeRange::UnspecifiedWithDuration(chrono::Duration::minutes(60));
        panel.room_ids = vec![1];

        assert!(!panel.is_scheduled());
    }

    #[test]
    fn test_is_scheduled_no_start_time() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "Test Panel".to_string();
        panel.timing = TimeRange::UnspecifiedWithDuration(chrono::Duration::minutes(60));
        panel.room_ids = vec![1];

        assert!(!panel.is_scheduled());
    }

    #[test]
    fn test_is_scheduled_no_room() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "Test Panel".to_string();
        panel.timing = TimeRange::UnspecifiedWithDuration(chrono::Duration::minutes(60));
        panel.room_ids = vec![];

        assert!(!panel.is_scheduled());
    }

    #[test]
    fn test_is_scheduled_zero_duration() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "Test Panel".to_string();
        panel.timing = TimeRange::UnspecifiedWithDuration(chrono::Duration::minutes(0));
        panel.room_ids = vec![1];

        assert!(!panel.is_scheduled());
    }

    #[test]
    fn test_is_scheduled_end_before_start() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "Test Panel".to_string();
        let start_time =
            chrono::NaiveDateTime::parse_from_str("2026-06-26T13:00:00", "%Y-%m-%dT%H:%M:%S")
                .unwrap();
        let end_time =
            chrono::NaiveDateTime::parse_from_str("2026-06-26T12:00:00", "%Y-%m-%dT%H:%M:%S")
                .unwrap();
        panel.timing = TimeRange::Scheduled {
            start_time,
            duration: end_time - start_time, // This will be negative
        }; // End before start
        panel.room_ids = vec![1];

        assert!(!panel.is_scheduled());
    }

    #[test]
    fn test_is_scheduled_no_duration_or_end() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "Test Panel".to_string();
        panel.timing = TimeRange::Unspecified; // No timing info
        panel.room_ids = vec![1];

        assert!(!panel.is_scheduled());
    }

    #[test]
    fn test_is_scheduled_whitespace_name() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "   ".to_string(); // Whitespace only
        panel.timing = TimeRange::Scheduled {
            start_time: chrono::NaiveDateTime::parse_from_str(
                "2026-06-26T12:00:00",
                "%Y-%m-%dT%H:%M:%S",
            )
            .unwrap(),
            duration: chrono::Duration::minutes(60),
        };
        panel.room_ids = vec![1];

        assert!(!panel.is_scheduled());
    }

    #[test]
    fn test_is_scheduled_invalid_datetime_format() {
        let mut panel = Panel::new("P001", "BASE001");
        panel.name = "Test Panel".to_string();
        panel.timing = TimeRange::Scheduled {
            start_time: chrono::NaiveDateTime::parse_from_str(
                "2026-06-26T12:00:00",
                "%Y-%m-%dT%H:%M:%S",
            )
            .unwrap(),
            duration: chrono::Duration::minutes(60),
        };
        panel.room_ids = vec![1];

        assert!(panel.is_scheduled());
    }
}
