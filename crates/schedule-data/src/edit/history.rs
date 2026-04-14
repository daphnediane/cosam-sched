/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Undo/redo history stack for [`EditCommand`]s.
//!
//! [`EditHistory`] maintains two stacks — undo and redo — that track the
//! sequence of applied commands.  Pushing a new command clears the redo
//! stack (standard linear undo model).

use super::command::EditCommand;

/// Linear undo/redo history.
///
/// Commands are pushed onto the undo stack after successful application.
/// Calling [`undo`](EditHistory::undo) pops from undo and pushes onto redo;
/// [`redo`](EditHistory::redo) does the reverse.  Any new push clears the
/// redo stack, discarding the "future" branch.
#[derive(Debug, Clone, Default)]
pub struct EditHistory {
    undo_stack: Vec<EditCommand>,
    redo_stack: Vec<EditCommand>,
}

impl EditHistory {
    /// Create an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a command onto the undo stack, clearing the redo stack.
    pub fn push(&mut self, command: EditCommand) {
        self.redo_stack.clear();
        self.undo_stack.push(command);
    }

    /// Pop the most recent command from the undo stack and push it onto redo.
    ///
    /// Returns the command so the caller can call `cmd.undo(schedule)`.
    /// Returns `None` when there is nothing to undo.
    pub fn undo(&mut self) -> Option<EditCommand> {
        let cmd = self.undo_stack.pop()?;
        self.redo_stack.push(cmd.clone());
        Some(cmd)
    }

    /// Pop the most recent command from the redo stack and push it onto undo.
    ///
    /// Returns the command so the caller can call `cmd.apply(schedule)`.
    /// Returns `None` when there is nothing to redo.
    pub fn redo(&mut self) -> Option<EditCommand> {
        let cmd = self.redo_stack.pop()?;
        self.undo_stack.push(cmd.clone());
        Some(cmd)
    }

    /// Whether an undo operation is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether a redo operation is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Number of commands on the undo stack.
    #[must_use]
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of commands on the redo stack.
    #[must_use]
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Read-only view of the undo stack (oldest first).
    pub fn undo_stack(&self) -> &[EditCommand] {
        &self.undo_stack
    }

    /// Read-only view of the redo stack (oldest first).
    pub fn redo_stack(&self) -> &[EditCommand] {
        &self.redo_stack
    }

    /// Clear both stacks.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
