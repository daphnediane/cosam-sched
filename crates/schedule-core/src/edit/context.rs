/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use crate::data::schedule::Schedule;

use super::command::EditCommand;
use super::history::EditHistory;

/// Wraps a mutable reference to a `Schedule` together with an `EditHistory`,
/// providing the primary API surface for all schedule mutations.
///
/// All mutations go through `EditContext` so that undo/redo is tracked
/// automatically.  Consumers should never mutate `Schedule` fields directly
/// when an `EditContext` is available.
pub struct EditContext<'a> {
    pub schedule: &'a mut Schedule,
    pub history: &'a mut EditHistory,
}

impl<'a> EditContext<'a> {
    pub fn new(schedule: &'a mut Schedule, history: &'a mut EditHistory) -> Self {
        Self { schedule, history }
    }

    /// Execute a command: apply it to the schedule and push it onto the undo
    /// stack.
    pub fn execute(&mut self, mut command: EditCommand) {
        command.apply(self.schedule);
        self.history.push(command);
    }

    /// Execute a batch of commands as a single undo step.
    pub fn execute_batch(&mut self, commands: Vec<EditCommand>) {
        let mut batch = EditCommand::Batch(commands);
        batch.apply(self.schedule);
        self.history.push(batch);
    }

    /// Undo the most recent command. Returns `true` if an undo was performed.
    pub fn undo(&mut self) -> bool {
        self.history.undo(self.schedule)
    }

    /// Redo the most recently undone command. Returns `true` if a redo was
    /// performed.
    pub fn redo(&mut self) -> bool {
        self.history.redo(self.schedule)
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
}
