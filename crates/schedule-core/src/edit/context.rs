/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use crate::data::schedule::Schedule;

use super::command::EditCommand;
use super::history::EditHistory;

/// Wraps a mutable reference to a `Schedule` together with an optional
/// `EditHistory`, providing the primary API surface for all schedule
/// mutations.
///
/// When `history` is `Some`, commands are recorded for undo/redo (editing
/// mode).  When `history` is `None`, commands are applied but not recorded
/// (import mode).
pub struct EditContext<'a> {
    pub schedule: &'a mut Schedule,
    history: Option<&'a mut EditHistory>,
}

impl<'a> EditContext<'a> {
    /// Create an editing context with undo/redo tracking.
    pub fn new(schedule: &'a mut Schedule, history: &'a mut EditHistory) -> Self {
        Self {
            schedule,
            history: Some(history),
        }
    }

    /// Create an import context without undo/redo tracking.
    /// Commands are applied directly; nothing is recorded.
    pub fn import(schedule: &'a mut Schedule) -> Self {
        Self {
            schedule,
            history: None,
        }
    }

    /// Execute a command: apply it to the schedule and (if tracking) push it
    /// onto the undo stack.
    pub fn execute(&mut self, mut command: EditCommand) {
        command.apply(self.schedule);
        if let Some(ref mut history) = self.history {
            history.push(command);
        }
    }

    /// Execute a batch of commands as a single undo step.
    pub fn execute_batch(&mut self, commands: Vec<EditCommand>) {
        let mut batch = EditCommand::Batch(commands);
        batch.apply(self.schedule);
        if let Some(ref mut history) = self.history {
            history.push(batch);
        }
    }

    /// Undo the most recent command. Returns `true` if an undo was performed.
    pub fn undo(&mut self) -> bool {
        match self.history {
            Some(ref mut history) => history.undo(self.schedule),
            None => false,
        }
    }

    /// Redo the most recently undone command. Returns `true` if a redo was
    /// performed.
    pub fn redo(&mut self) -> bool {
        match self.history {
            Some(ref mut history) => history.redo(self.schedule),
            None => false,
        }
    }

    pub fn can_undo(&self) -> bool {
        self.history.as_ref().map(|h| h.can_undo()).unwrap_or(false)
    }

    pub fn can_redo(&self) -> bool {
        self.history.as_ref().map(|h| h.can_redo()).unwrap_or(false)
    }
}
