/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`EditHistory`] stack for undo/redo.

use crate::edit::command::EditCommand;
use std::collections::VecDeque;

// ── EditHistory ───────────────────────────────────────────────────────────────

/// Stack-based undo/redo history for [`EditCommand`]s.
///
/// `apply` pushes to the undo stack (dropping oldest entry when at capacity)
/// and clears the redo stack.  `undo` and `redo` move the top entry between
/// the two stacks.
#[derive(Debug)]
pub struct EditHistory {
    undo_stack: VecDeque<EditCommand>,
    redo_stack: VecDeque<EditCommand>,
    max_depth: usize,
}

impl EditHistory {
    /// Default maximum undo/redo depth.
    pub const DEFAULT_MAX_DEPTH: usize = 100;

    /// Create a new history with the given maximum depth.
    #[must_use]
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_depth,
        }
    }

    /// Push an already-executed command's inverse onto the undo stack.
    ///
    /// The caller supplies the *inverse* of the command that was executed
    /// (i.e. what needs to run on undo).  Clears the redo stack.
    pub(crate) fn push_undo(&mut self, inverse: EditCommand) {
        if self.undo_stack.len() == self.max_depth {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(inverse);
        self.redo_stack.clear();
    }

    /// Pop the most recent undo inverse.
    pub(crate) fn pop_undo(&mut self) -> Option<EditCommand> {
        self.undo_stack.pop_back()
    }

    /// Push a re-executable command onto the redo stack.
    pub(crate) fn push_redo(&mut self, cmd: EditCommand) {
        self.redo_stack.push_back(cmd);
    }

    /// Pop the most recent redo command.
    pub(crate) fn pop_redo(&mut self) -> Option<EditCommand> {
        self.redo_stack.pop_back()
    }

    /// Number of operations that can currently be undone.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of operations that can currently be redone.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for EditHistory {
    fn default() -> Self {
        Self::with_max_depth(Self::DEFAULT_MAX_DEPTH)
    }
}
