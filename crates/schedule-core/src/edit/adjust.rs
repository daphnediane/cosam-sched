/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use crate::data::panel::ExtraValue;

use super::command::{EditCommand, PanelField, SessionField, SessionScheduleState};
use super::context::EditContext;

impl EditContext<'_> {
    /// Set an `Option<String>` field on a panel.
    pub fn set_panel_field(&mut self, panel_id: &str, field: PanelField, value: Option<String>) {
        let cmd = EditCommand::SetPanelField {
            panel_id: panel_id.to_string(),
            field,
            old: None, // filled by apply
            new: value,
        };
        self.execute(cmd);
    }

    /// Set the panel name (a required `String` field, not `Option<String>`).
    pub fn set_panel_name(&mut self, panel_id: &str, name: &str) {
        let cmd = EditCommand::SetPanelName {
            panel_id: panel_id.to_string(),
            old: String::new(), // filled by apply
            new: name.to_string(),
        };
        self.execute(cmd);
    }

    /// Set a boolean field on a panel.
    pub fn set_panel_bool(&mut self, panel_id: &str, field_name: &str, value: bool) {
        let cmd = EditCommand::SetPanelBool {
            panel_id: panel_id.to_string(),
            field_name: field_name.to_string(),
            old: false, // filled by apply
            new: value,
        };
        self.execute(cmd);
    }

    /// Set an `Option<String>` field on a session.
    pub fn set_session_field(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
        field: SessionField,
        value: Option<String>,
    ) {
        let cmd = EditCommand::SetSessionField {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            field,
            old: None, // filled by apply
            new: value,
        };
        self.execute(cmd);
    }

    /// Set the duration of a session.
    pub fn set_session_duration(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
        duration: u32,
    ) {
        let cmd = EditCommand::SetSessionDuration {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            old: 0, // filled by apply
            new: duration,
        };
        self.execute(cmd);
    }

    /// Add a credited presenter to a session.
    pub fn add_presenter_to_session(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
        name: &str,
    ) {
        // Check for duplicate first
        let already_present = self
            .schedule
            .panels
            .get(panel_id)
            .and_then(|p| p.parts.get(part_index))
            .and_then(|pt| pt.sessions.get(session_index))
            .is_some_and(|s| {
                s.credited_presenters
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(name))
            });

        if already_present {
            return;
        }

        let cmd = EditCommand::AddPresenterToSession {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            name: name.to_string(),
        };
        self.execute(cmd);
    }

    /// Remove a credited presenter from a session.
    pub fn remove_presenter_from_session(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
        name: &str,
    ) {
        let cmd = EditCommand::RemovePresenterFromSession {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            name: name.to_string(),
            position: 0, // filled by apply
        };
        self.execute(cmd);
    }

    /// Reschedule a session with new room/time/duration.
    pub fn reschedule_session(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
        new_state: SessionScheduleState,
    ) {
        let cmd = EditCommand::RescheduleSession {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            old_state: SessionScheduleState {
                room_ids: Vec::new(),
                start_time: None,
                end_time: None,
                duration: 0,
            }, // filled by apply
            new_state,
        };
        self.execute(cmd);
    }

    /// Set a metadata key on a session.
    pub fn set_session_metadata(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
        key: &str,
        value: ExtraValue,
    ) {
        let cmd = EditCommand::SetSessionMetadata {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            key: key.to_string(),
            old: None, // filled by apply
            new: value,
        };
        self.execute(cmd);
    }

    /// Clear a metadata key from a session.
    pub fn clear_session_metadata(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
        key: &str,
    ) {
        // Only emit if the key exists
        let has_key = self
            .schedule
            .panels
            .get(panel_id)
            .and_then(|p| p.parts.get(part_index))
            .and_then(|pt| pt.sessions.get(session_index))
            .is_some_and(|s| s.metadata.contains_key(key));

        if !has_key {
            return;
        }

        let cmd = EditCommand::ClearSessionMetadata {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            key: key.to_string(),
            old: ExtraValue::String(String::new()), // filled by apply
        };
        self.execute(cmd);
    }
}
