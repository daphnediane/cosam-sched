/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use crate::data::panel::ExtraValue;

use super::command::{EditCommand, PanelField, SessionScheduleState};
use super::context::EditContext;

/// Convenience re-export so callers can still spell `SessionField` even though
/// it is now just an alias for `PanelField`.
pub use super::command::SessionField;

impl EditContext<'_> {
    // ── Panel-level field setters ────────────────────────────────────────────

    /// Set an `Option<String>` field on a flat Panel (addressed by full Uniq ID).
    pub fn set_panel_field(&mut self, panel_id: &str, field: PanelField, value: Option<String>) {
        self.execute(EditCommand::SetPanelField {
            panel_id: panel_id.to_string(),
            field,
            old: None,
            new: value,
        });
    }

    /// Set the name field on a flat Panel.
    pub fn set_panel_name(&mut self, panel_id: &str, name: &str) {
        self.execute(EditCommand::SetPanelName {
            panel_id: panel_id.to_string(),
            old: String::new(),
            new: name.to_string(),
        });
    }

    /// Set a boolean field on a flat Panel.
    pub fn set_panel_bool(&mut self, panel_id: &str, field_name: &str, value: bool) {
        self.execute(EditCommand::SetPanelBool {
            panel_id: panel_id.to_string(),
            field_name: field_name.to_string(),
            old: false,
            new: value,
        });
    }

    /// Set the duration of a flat Panel.
    pub fn set_panel_duration(&mut self, panel_id: &str, duration: u32) {
        self.execute(EditCommand::SetPanelDuration {
            panel_id: panel_id.to_string(),
            old: None,
            new: Some(chrono::Duration::minutes(duration as i64)),
        });
    }

    // ── Backward-compat wrappers for callers that still use session addressing ──
    //
    // In the flat model a "session" is just a Panel.  The `panel_id` here must
    // already be the full Uniq ID of the target flat Panel; `part_index` and
    // `session_index` are ignored (kept for API stability while callers migrate).

    // ── Presenters ───────────────────────────────────────────────────────────

    /// Add a credited presenter to a flat Panel (deduplicated).
    pub fn add_presenter_to_panel(&mut self, panel_id: &str, name: &str) {
        let already_present = self
            .schedule
            .panel_sets
            .values()
            .flat_map(|ps| ps.panels.iter())
            .find(|p| p.id == panel_id)
            .is_some_and(|p| {
                p.credited_presenters
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(name))
            });

        if already_present {
            return;
        }

        self.execute(EditCommand::AddPresenterToPanel {
            panel_id: panel_id.to_string(),
            name: name.to_string(),
        });
    }

    /// Remove a credited presenter from a flat Panel.
    pub fn remove_presenter_from_panel(&mut self, panel_id: &str, name: &str) {
        self.execute(EditCommand::RemovePresenterFromPanel {
            panel_id: panel_id.to_string(),
            name: name.to_string(),
            position: 0,
        });
    }

    // ── Scheduling ───────────────────────────────────────────────────────────

    /// Reschedule a flat Panel with new room / time / duration.
    pub fn reschedule_panel(&mut self, panel_id: &str, new_state: SessionScheduleState) {
        self.execute(EditCommand::ReschedulePanel {
            panel_id: panel_id.to_string(),
            old_state: SessionScheduleState {
                room_ids: Vec::new(),
                timing: crate::data::time::TimeRange::Unspecified,
            },
            new_state,
        });
    }

    // ── Metadata ─────────────────────────────────────────────────────────────

    /// Set a metadata key on a flat Panel.
    pub fn set_panel_metadata(&mut self, panel_id: &str, key: &str, value: ExtraValue) {
        self.execute(EditCommand::SetPanelMetadata {
            panel_id: panel_id.to_string(),
            key: key.to_string(),
            old: None,
            new: value,
        });
    }

    /// Clear a metadata key from a flat Panel.
    pub fn clear_panel_metadata(&mut self, panel_id: &str, key: &str) {
        let has_key = self
            .schedule
            .panel_sets
            .values()
            .flat_map(|ps| ps.panels.iter())
            .find(|p| p.id == panel_id)
            .is_some_and(|p| p.metadata.contains_key(key));

        if !has_key {
            return;
        }
        self.execute(EditCommand::ClearPanelMetadata {
            panel_id: panel_id.to_string(),
            key: key.to_string(),
            old: ExtraValue::String(String::new()),
        });
    }

    /// Set a metadata key on a room.
    pub fn set_room_metadata(&mut self, uid: u32, key: &str, value: ExtraValue) {
        self.execute(EditCommand::SetRoomMetadata {
            uid,
            key: key.to_string(),
            old: None,
            new: value,
        });
    }

    /// Clear a metadata key from a room.
    pub fn clear_room_metadata(&mut self, uid: u32, key: &str) {
        let has_key = self
            .schedule
            .rooms
            .iter()
            .find(|r| r.uid == uid)
            .and_then(|r| r.metadata.as_ref())
            .is_some_and(|m| m.contains_key(key));

        if !has_key {
            return;
        }
        self.execute(EditCommand::ClearRoomMetadata {
            uid,
            key: key.to_string(),
            old: ExtraValue::String(String::new()),
        });
    }

    /// Set a metadata key on a panel type.
    pub fn set_panel_type_metadata(&mut self, prefix: &str, key: &str, value: ExtraValue) {
        self.execute(EditCommand::SetPanelTypeMetadata {
            prefix: prefix.to_string(),
            key: key.to_string(),
            old: None,
            new: value,
        });
    }

    /// Clear a metadata key from a panel type.
    pub fn clear_panel_type_metadata(&mut self, prefix: &str, key: &str) {
        let has_key = self
            .schedule
            .panel_types
            .get(prefix)
            .and_then(|pt| pt.metadata.as_ref())
            .is_some_and(|m| m.contains_key(key));

        if !has_key {
            return;
        }
        self.execute(EditCommand::ClearPanelTypeMetadata {
            prefix: prefix.to_string(),
            key: key.to_string(),
            old: ExtraValue::String(String::new()),
        });
    }

    // ── Presenter lists ──────────────────────────────────────────────────────

    /// Replace the entire credited presenter list on a flat Panel.
    pub fn set_panel_presenters(&mut self, panel_id: &str, presenters: Vec<String>) {
        self.execute(EditCommand::SetPanelPresenters {
            panel_id: panel_id.to_string(),
            old: Vec::new(),
            new: presenters,
        });
    }
}
