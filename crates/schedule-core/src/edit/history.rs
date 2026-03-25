/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Serialize};

use super::command::EditCommand;
use crate::data::schedule::Schedule;

const DEFAULT_MAX_UNDO: usize = 50;

/// Manages undo/redo stacks of `EditCommand`s.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditHistory {
    undo_stack: Vec<EditCommand>,
    redo_stack: Vec<EditCommand>,
    max_depth: usize,
}

impl EditHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth: DEFAULT_MAX_UNDO,
        }
    }

    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    /// Push a command onto the undo stack, clearing the redo stack.
    pub fn push(&mut self, command: EditCommand) {
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(command);
        self.redo_stack.clear();
    }

    /// Undo the most recent command, moving it to the redo stack.
    /// Returns `true` if an undo was performed.
    pub fn undo(&mut self, schedule: &mut Schedule) -> bool {
        let Some(command) = self.undo_stack.pop() else {
            return false;
        };
        command.undo(schedule);
        self.redo_stack.push(command);
        true
    }

    /// Redo the most recently undone command, moving it back to the undo stack.
    /// Returns `true` if a redo was performed.
    pub fn redo(&mut self, schedule: &mut Schedule) -> bool {
        let Some(mut command) = self.redo_stack.pop() else {
            return false;
        };
        command.apply(schedule);
        self.undo_stack.push(command);
        true
    }

    /// Returns true if there are commands to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are commands to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Number of undo steps available.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of redo steps available.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Returns true if both stacks are empty.
    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty() && self.redo_stack.is_empty()
    }

    pub fn undo_stack(&self) -> &[EditCommand] {
        &self.undo_stack
    }

    pub fn redo_stack(&self) -> &[EditCommand] {
        &self.redo_stack
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
}

impl Default for EditHistory {
    fn default() -> Self {
        Self::new()
    }
}
