/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edit context combining a [`Schedule`] with an optional [`EditHistory`].
//!
//! [`EditContext`] is the primary entry point for making tracked mutations.
//! It wraps a schedule and an optional history so that callers can execute
//! commands, undo, and redo without manually managing the history stack.

use super::command::EditCommand;
use super::history::EditHistory;
use crate::field::FieldError;
use crate::schedule::Schedule;

/// A schedule paired with an optional undo/redo history.
///
/// When `history` is `Some`, every successful [`execute`](Self::execute)
/// pushes the command onto the undo stack.  When `None`, commands are
/// applied but not recorded (fire-and-forget mode, useful for batch
/// imports).
///
/// `dirty` is set to `true` on any successful [`execute`] and cleared by
/// [`mark_clean`](Self::mark_clean).  Use it to drive save prompts.
#[derive(Debug)]
pub struct EditContext {
    schedule: Schedule,
    history: Option<EditHistory>,
    dirty: bool,
}

impl EditContext {
    /// Create a context with history tracking enabled.
    pub fn new(schedule: Schedule) -> Self {
        Self {
            schedule,
            history: Some(EditHistory::new()),
            dirty: false,
        }
    }

    /// Create a context without history tracking.
    pub fn without_history(schedule: Schedule) -> Self {
        Self {
            schedule,
            history: None,
            dirty: false,
        }
    }

    /// Borrow the inner schedule.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Mutably borrow the inner schedule.
    ///
    /// **Warning:** direct mutations bypass the history. Prefer
    /// [`execute`](Self::execute) for tracked changes.
    pub fn schedule_mut(&mut self) -> &mut Schedule {
        &mut self.schedule
    }

    /// Borrow the history (if tracking is enabled).
    pub fn history(&self) -> Option<&EditHistory> {
        self.history.as_ref()
    }

    // ------------------------------------------------------------------
    // Command execution
    // ------------------------------------------------------------------

    /// Whether the schedule has unsaved changes.
    ///
    /// Set to `true` on every successful [`execute`](Self::execute); cleared
    /// by [`mark_clean`](Self::mark_clean).
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the context as clean (e.g., after a successful save).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Apply a single command, recording it in history if enabled.
    pub fn execute(&mut self, command: EditCommand) -> Result<(), FieldError> {
        command.apply(&mut self.schedule)?;
        self.dirty = true;
        if let Some(ref mut history) = self.history {
            history.push(command);
        }
        Ok(())
    }

    /// Apply a batch of commands as a single compound undo step.
    pub fn execute_batch(
        &mut self,
        label: impl Into<String>,
        commands: Vec<EditCommand>,
    ) -> Result<(), FieldError> {
        let compound = EditCommand::compound(label, commands);
        self.execute(compound)
    }

    // ------------------------------------------------------------------
    // Undo / Redo
    // ------------------------------------------------------------------

    /// Undo the most recent command. Returns `false` if nothing to undo or
    /// if history tracking is disabled.
    pub fn undo(&mut self) -> Result<bool, FieldError> {
        let Some(ref mut history) = self.history else {
            return Ok(false);
        };
        let Some(cmd) = history.undo() else {
            return Ok(false);
        };
        cmd.undo(&mut self.schedule)?;
        Ok(true)
    }

    /// Redo the most recently undone command. Returns `false` if nothing to
    /// redo or if history tracking is disabled.
    pub fn redo(&mut self) -> Result<bool, FieldError> {
        let Some(ref mut history) = self.history else {
            return Ok(false);
        };
        let Some(cmd) = history.redo() else {
            return Ok(false);
        };
        cmd.apply(&mut self.schedule)?;
        Ok(true)
    }

    /// Whether an undo is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history.as_ref().is_some_and(EditHistory::can_undo)
    }

    /// Whether a redo is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.history.as_ref().is_some_and(EditHistory::can_redo)
    }
}
