/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use crate::data::source_info::ChangeState;

use super::command::{EditCommand, SessionScheduleState};
use super::context::EditContext;

impl EditContext<'_> {
    /// Unschedule a session: clear its room, start/end times, but keep the
    /// session itself intact.
    pub fn unschedule_session(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
    ) {
        let cmd = EditCommand::UnscheduleSession {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            old_state: SessionScheduleState {
                room_ids: Vec::new(),
                start_time: None,
                end_time: None,
                duration: 0,
            }, // filled by apply
        };
        self.execute(cmd);
    }

    /// Soft-delete a session: mark it as `ChangeState::Deleted`.
    /// The session remains in memory until `post_save_cleanup` removes it.
    pub fn soft_delete_session(
        &mut self,
        panel_id: &str,
        part_index: usize,
        session_index: usize,
    ) {
        let cmd = EditCommand::SoftDeleteSession {
            panel_id: panel_id.to_string(),
            part_index,
            session_index,
            old_change_state: ChangeState::Unchanged, // filled by apply
        };
        self.execute(cmd);
    }

    /// Soft-delete an entire panel and all its sessions.
    pub fn soft_delete_panel(&mut self, panel_id: &str) {
        let cmd = EditCommand::SoftDeletePanel {
            panel_id: panel_id.to_string(),
            old_change_state: ChangeState::Unchanged, // filled by apply
        };
        self.execute(cmd);
    }
}
