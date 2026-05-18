/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`EditHistory`] stack for undo/redo.

use std::borrow::Cow;
use std::collections::VecDeque;

// ── UndoEntry ─────────────────────────────────────────────────────────────────

/// A single reversible step in the edit history.
///
/// Rather than storing command inverses, each entry records the CRDT heads
/// *before* the step was applied (`pre_heads`) and the raw automerge change
/// bytes produced by the step (`changes`).
///
/// - **Undo**: fork the document back to `pre_heads` and rebuild the cache.
/// - **Redo**: call `apply_changes(changes)` and rebuild the cache.
///
/// The same `UndoEntry` is pushed to both the undo and redo stacks — the
/// operation differs, but the stored data is identical.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    /// Human-readable description of the action, suitable for display in
    /// "Undo <label>" / "Redo <label>" menu items.
    pub label: Cow<'static, str>,
    /// CRDT document heads immediately *before* this step was applied.
    /// Used as the `fork_at` target when undoing.
    pub pre_heads: Vec<automerge::ChangeHash>,
    /// Raw automerge change bytes produced by this step (i.e. the delta
    /// between `pre_heads` and the post-step heads).  Used by redo to
    /// reapply the step via `apply_changes`.
    pub changes: Vec<Vec<u8>>,
}

// ── EditHistory ───────────────────────────────────────────────────────────────

/// Stack-based undo/redo history for [`UndoEntry`]s.
///
/// `push_undo` appends to the undo stack (dropping the oldest entry when at
/// capacity) and clears the redo stack.  `undo` and `redo` move the top entry
/// between the two stacks; the caller is responsible for actually applying the
/// fork or replay to the schedule.
#[derive(Debug)]
pub struct EditHistory {
    undo_stack: VecDeque<UndoEntry>,
    redo_stack: VecDeque<UndoEntry>,
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

    /// Push an entry onto the undo stack.
    ///
    /// If the stack is at capacity the oldest entry is discarded.
    /// Does **not** clear the redo stack — call [`clear_redo`](Self::clear_redo)
    /// explicitly when a new user action should invalidate the redo branch.
    pub(crate) fn push_undo(&mut self, entry: UndoEntry) {
        if self.undo_stack.len() == self.max_depth {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(entry);
    }

    /// Clear the redo stack.
    ///
    /// Called by [`EditContext::apply`] after a new user action so that the
    /// now-invalidated redo branch is discarded.
    pub(crate) fn clear_redo(&mut self) {
        self.redo_stack.clear();
    }

    /// Pop the most recent undo entry.
    pub(crate) fn pop_undo(&mut self) -> Option<UndoEntry> {
        self.undo_stack.pop_back()
    }

    /// Push an entry onto the redo stack (does *not* clear the undo stack).
    pub(crate) fn push_redo(&mut self, entry: UndoEntry) {
        self.redo_stack.push_back(entry);
    }

    /// Pop the most recent redo entry.
    pub(crate) fn pop_redo(&mut self) -> Option<UndoEntry> {
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

    /// Label of the next operation that would be undone, if any.
    #[must_use]
    pub fn undo_label(&self) -> Option<&str> {
        self.undo_stack.back().map(|e| e.label.as_ref())
    }

    /// Label of the next operation that would be redone, if any.
    #[must_use]
    pub fn redo_label(&self) -> Option<&str> {
        self.redo_stack.back().map(|e| e.label.as_ref())
    }
}

impl Default for EditHistory {
    fn default() -> Self {
        Self::with_max_depth(Self::DEFAULT_MAX_DEPTH)
    }
}
