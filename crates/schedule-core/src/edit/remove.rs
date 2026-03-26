/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use crate::data::source_info::ChangeState;

use super::command::{EditCommand, SessionScheduleState};
use super::context::EditContext;

impl EditContext<'_> {
    /// Unschedule a flat Panel: clear its room, start/end times.
    pub fn unschedule_panel(&mut self, panel_id: &str) {
        self.execute(EditCommand::UnschedulePanel {
            panel_id: panel_id.to_string(),
            old_state: SessionScheduleState {
                room_ids: Vec::new(),
                start_time: None,
                end_time: None,
                duration: 0,
            },
        });
    }

    /// *Deprecated alias* — use [`unschedule_panel`] directly.
    #[allow(unused_variables)]
    pub fn unschedule_session(&mut self, panel_id: &str, part_index: usize, session_index: usize) {
        self.unschedule_panel(panel_id);
    }

    /// Soft-delete a single flat Panel: marks it `ChangeState::Deleted`.
    pub fn soft_delete_panel(&mut self, panel_id: &str) {
        self.execute(EditCommand::SoftDeletePanel {
            panel_id: panel_id.to_string(),
            old_change_state: ChangeState::Unchanged,
        });
    }

    /// *Deprecated alias* for [`soft_delete_panel`].
    #[allow(unused_variables)]
    pub fn soft_delete_session(&mut self, panel_id: &str, part_index: usize, session_index: usize) {
        self.soft_delete_panel(panel_id);
    }

    /// Soft-delete every Panel in a PanelSet (addressed by base ID).
    pub fn soft_delete_panel_set(&mut self, base_id: &str) {
        self.execute(EditCommand::SoftDeletePanelSet {
            base_id: base_id.to_string(),
            old_change_states: Vec::new(),
        });
    }
}
