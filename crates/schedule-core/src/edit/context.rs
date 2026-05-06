/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`EditContext`] facade for edit operations.

use crate::edit::command::{find_registration, EditCommand, EditError};
use crate::edit::history::EditHistory;
use crate::entity::{DynamicEntityId, EntityUuid, RuntimeEntityId};
use crate::schedule::Schedule;
use crate::value::FieldValue;

// ── EditContext ───────────────────────────────────────────────────────────────

/// Top-level facade that owns a [`Schedule`] and its [`EditHistory`].
///
/// All mutations to the schedule must go through this type so that every
/// change is tracked and reversible.
///
/// ## Dirty state
///
/// [`EditContext`] tracks whether the schedule has unsaved changes via a
/// simple counter: every successful [`apply`](Self::apply) increments it;
/// [`mark_clean`](Self::mark_clean) resets it to zero.
/// [`is_dirty`](Self::is_dirty) returns `true` when the counter is non-zero.
#[derive(Debug)]
pub struct EditContext {
    pub(crate) schedule: Schedule,
    pub(crate) history: EditHistory,
    dirty_count: usize,
}

impl EditContext {
    /// Create an `EditContext` wrapping an existing schedule.
    #[must_use]
    pub fn new(schedule: Schedule) -> Self {
        Self {
            schedule,
            history: EditHistory::default(),
            dirty_count: 0,
        }
    }

    /// Create an `EditContext` with a custom history depth.
    #[must_use]
    pub fn with_history_depth(schedule: Schedule, max_depth: usize) -> Self {
        Self {
            schedule,
            history: EditHistory::with_max_depth(max_depth),
            dirty_count: 0,
        }
    }

    /// Borrow the underlying schedule for read-only access.
    #[must_use]
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Mutably borrow the underlying schedule.
    ///
    /// Intended for callers that need to stamp metadata (e.g. `metadata.generator`)
    /// before saving.  Mutations made directly through this accessor bypass the edit
    /// history — use [`apply`](Self::apply) for all data edits.
    pub fn schedule_mut(&mut self) -> &mut Schedule {
        &mut self.schedule
    }

    /// Returns `true` if there are unsaved changes since the last
    /// [`mark_clean`](Self::mark_clean) call.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty_count > 0
    }

    /// Reset the dirty counter, marking the current state as saved.
    pub fn mark_clean(&mut self) {
        self.dirty_count = 0;
    }

    /// Returns the current depth of the undo stack.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.history.undo_depth()
    }

    /// Returns the current depth of the redo stack.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.history.redo_depth()
    }

    /// Execute a command, push its inverse onto the undo stack, and increment
    /// the dirty counter.
    ///
    /// On error the schedule is left in whatever state the partial execution
    /// reached (commands that have already been applied are not rolled back).
    /// The history is not modified on error.
    pub fn apply(&mut self, cmd: EditCommand) -> Result<(), EditError> {
        let inverse = cmd.execute(&mut self.schedule)?;
        self.history.push_undo(inverse);
        self.dirty_count += 1;
        self.schedule.touch_modified();
        Ok(())
    }

    /// Undo the most recent operation.
    ///
    /// Returns [`EditError::NothingToUndo`] if the undo stack is empty.
    pub fn undo(&mut self) -> Result<(), EditError> {
        let undo_cmd = self.history.pop_undo().ok_or(EditError::NothingToUndo)?;
        let redo_cmd = undo_cmd.execute(&mut self.schedule)?;
        self.history.push_redo(redo_cmd);
        self.dirty_count = self.dirty_count.saturating_sub(1);
        self.schedule.touch_modified();
        Ok(())
    }

    /// Redo the most recently undone operation.
    ///
    /// Returns [`EditError::NothingToRedo`] if the redo stack is empty.
    pub fn redo(&mut self) -> Result<(), EditError> {
        let redo_cmd = self.history.pop_redo().ok_or(EditError::NothingToRedo)?;
        let inverse = redo_cmd.execute(&mut self.schedule)?;
        self.history.push_undo(inverse);
        self.dirty_count += 1;
        self.schedule.touch_modified();
        Ok(())
    }

    // ── Convenience constructors ──────────────────────────────────────────────

    /// Build an `UpdateField` command that captures the current field value as
    /// `old_value` before writing `new_value`.
    ///
    /// Returns `Err` if the entity is not found or the field is write-only.
    pub fn update_field_cmd(
        &self,
        entity: impl DynamicEntityId,
        field: &'static str,
        new_value: FieldValue,
    ) -> Result<EditCommand, EditError> {
        let entity = RuntimeEntityId::from_dynamic(entity);
        let reg = find_registration(entity)?;
        let old_value = (reg.read_field_fn)(&self.schedule, entity.entity_uuid(), field)
            .map_err(|source| EditError::FieldRead {
                entity,
                field,
                source: Box::new(source),
            })?
            .ok_or(EditError::EntityNotFound(entity))?;
        Ok(EditCommand::UpdateField {
            entity,
            field,
            old_value,
            new_value,
        })
    }

    /// Build a `RemoveEntity` command that snapshots the entity's read+write
    /// fields before removal.
    ///
    /// Returns `Err` if the entity type is not registered.
    pub fn remove_entity_cmd(
        &self,
        entity: impl DynamicEntityId,
    ) -> Result<EditCommand, EditError> {
        let runtime_id = RuntimeEntityId::from_dynamic(entity);
        let reg = find_registration(runtime_id)?;
        let fields = (reg.snapshot_fn)(&self.schedule, runtime_id.entity_uuid());
        Ok(EditCommand::RemoveEntity {
            entity: runtime_id,
            fields,
        })
    }

    /// Build a `MovePanel` command (a `BatchEdit` of two `UpdateField`s).
    ///
    /// `time_field` and `room_field` are the canonical field names for the
    /// panel's start time and room respectively.
    pub fn move_panel_cmd(
        &self,
        panel: impl DynamicEntityId,
        time_field: &'static str,
        new_time: FieldValue,
        room_field: &'static str,
        new_room: FieldValue,
    ) -> Result<EditCommand, EditError> {
        let time_cmd = self.update_field_cmd(panel, time_field, new_time)?;
        let room_cmd = self.update_field_cmd(panel, room_field, new_room)?;
        Ok(EditCommand::MovePanel(Box::new(EditCommand::BatchEdit(
            vec![time_cmd, room_cmd],
        ))))
    }

    /// Build an `AddToField` command to add items to an edge field.
    ///
    /// This is a trivial constructor - no pre-read needed. The delta
    /// (actually added items) is captured during execute.
    pub fn add_to_field_cmd(
        &self,
        near: impl DynamicEntityId,
        edge: crate::edge::id::FullEdge,
        items: FieldValue,
    ) -> EditCommand {
        let near = RuntimeEntityId::from_dynamic(near);
        EditCommand::AddToField { near, edge, items }
    }

    /// Build a `RemoveFromField` command to remove items from an edge field.
    ///
    /// This is a trivial constructor - no pre-read needed. The delta
    /// (actually removed items) is captured during execute.
    pub fn remove_from_field_cmd(
        &self,
        near: impl DynamicEntityId,
        edge: crate::edge::id::FullEdge,
        items: FieldValue,
    ) -> EditCommand {
        let near = RuntimeEntityId::from_dynamic(near);
        EditCommand::RemoveFromField { near, edge, items }
    }
}
