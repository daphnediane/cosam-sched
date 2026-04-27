/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edit command system — [`EditCommand`], [`EditHistory`], and [`EditContext`].
//!
//! All mutations to the schedule go through this module.  Each change is
//! captured as a reversible [`EditCommand`], enabling undo/redo via
//! [`EditHistory`].  [`EditContext`] is the top-level facade that owns both a
//! [`Schedule`] and an [`EditHistory`] and provides the public mutation API.
//!
//! ## Key design properties
//!
//! - **Data-only commands**: every variant stores only [`RuntimeEntityId`],
//!   `&'static str` field names, and [`FieldValue`].  No closures or
//!   `Box<dyn Any>`.
//! - **`EditCommand: Clone`**: all stored types are `Copy`/`Clone`.
//! - **Field selection**: `AddEntity` and `RemoveEntity` snapshots contain
//!   only fields that are both readable *and* writable (i.e.
//!   `read_fn.is_some() && write_fn.is_some()`).  Read-only computed fields
//!   and write-only modifier fields are excluded.
//! - **Stable identity**: `AddEntity` redo and `RemoveEntity` undo always
//!   recreate the entity with its original UUID via `UuidPreference::Exact`.
//! - **CRDT hook**: every applied command passes through [`EditContext::apply`],
//!   which is the natural integration point for generating CRDT operations
//!   in Phase 4.

use crate::builder::BuildError;
use crate::entity::{
    registered_entity_types, DynamicEntityId, EntityTyped, EntityUuid, RuntimeEntityId,
};
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue};
use std::collections::VecDeque;
use thiserror::Error;

// ── EditError ─────────────────────────────────────────────────────────────────

/// Errors produced by the edit command system.
#[derive(Debug, Error)]
pub enum EditError {
    /// No registered entity type matches the given type name.
    #[error("unknown entity type: {0:?}")]
    UnknownEntityType(&'static str),

    /// The entity UUID was not found in any type's storage.
    #[error("entity not found: {0}")]
    EntityNotFound(RuntimeEntityId),

    /// A field read failed.
    #[error("field read error on {entity}, field {field:?}: {source}")]
    FieldRead {
        entity: RuntimeEntityId,
        field: &'static str,
        #[source]
        source: Box<FieldError>,
    },

    /// A field write failed.
    #[error("field write error on {entity}, field {field:?}: {source}")]
    FieldWrite {
        entity: RuntimeEntityId,
        field: &'static str,
        #[source]
        source: Box<FieldError>,
    },

    /// An `AddEntity` or `RemoveEntity` rebuild failed.
    #[error("entity build error on {entity}: {source}")]
    Build {
        entity: RuntimeEntityId,
        #[source]
        source: Box<BuildError>,
    },

    /// Undo stack is empty.
    #[error("nothing to undo")]
    NothingToUndo,

    /// Redo stack is empty.
    #[error("nothing to redo")]
    NothingToRedo,
}

// ── EditCommand ───────────────────────────────────────────────────────────────

/// A reversible mutation to a [`Schedule`].
///
/// All variants store only data (IDs, field names, values); no closures or
/// type-erased heap allocations.  This makes `EditCommand: Clone` and means
/// commands can be serialized for logging or CRDT broadcast.
///
/// Construct commands via [`EditContext`] helper methods rather than directly,
/// so that old values are captured automatically.
#[derive(Debug, Clone)]
pub enum EditCommand {
    /// Change a single field on an existing entity.
    ///
    /// `old_value` is the value read immediately before the write; it is used
    /// to reverse the change on undo.
    UpdateField {
        entity: RuntimeEntityId,
        field: &'static str,
        old_value: FieldValue,
        new_value: FieldValue,
    },

    /// Create a new entity with the given field values.
    ///
    /// The `entity` id carries the exact UUID so that redo recreates the same
    /// identity.  `fields` contains only read+write field snapshots.
    AddEntity {
        entity: RuntimeEntityId,
        fields: Vec<(&'static str, FieldValue)>,
    },

    /// Remove an existing entity.
    ///
    /// `fields` is the read+write snapshot captured immediately before
    /// removal, used to restore the entity on undo.
    RemoveEntity {
        entity: RuntimeEntityId,
        fields: Vec<(&'static str, FieldValue)>,
    },

    /// Move a panel to a new time slot and/or room.
    ///
    /// Stored as a [`BatchEdit`] of two [`UpdateField`] commands.
    ///
    /// [`BatchEdit`]: EditCommand::BatchEdit
    /// [`UpdateField`]: EditCommand::UpdateField
    MovePanel(Box<EditCommand>),

    /// Execute a sequence of commands as a single atomic undo/redo unit.
    BatchEdit(Vec<EditCommand>),
}

impl EditCommand {
    /// Apply this command to the given schedule, returning its inverse.
    fn execute(self, schedule: &mut Schedule) -> Result<EditCommand, EditError> {
        match self {
            EditCommand::UpdateField {
                entity,
                field,
                old_value,
                new_value,
            } => {
                let reg = find_registration(entity)?;
                (reg.write_field_fn)(schedule, entity.entity_uuid(), field, new_value.clone())
                    .map_err(|source| EditError::FieldWrite {
                        entity,
                        field,
                        source: Box::new(source),
                    })?;
                Ok(EditCommand::UpdateField {
                    entity,
                    field,
                    old_value: new_value,
                    new_value: old_value,
                })
            }

            EditCommand::AddEntity { entity, ref fields } => {
                let reg = find_registration(entity)?;
                (reg.build_fn)(schedule, entity.entity_uuid(), fields).map_err(|source| {
                    EditError::Build {
                        entity,
                        source: Box::new(source),
                    }
                })?;
                let fields_snapshot = fields.clone();
                Ok(EditCommand::RemoveEntity {
                    entity,
                    fields: fields_snapshot,
                })
            }

            EditCommand::RemoveEntity { entity, ref fields } => {
                let reg = find_registration(entity)?;
                let fields_snapshot = fields.clone();
                (reg.remove_fn)(schedule, entity.entity_uuid());
                Ok(EditCommand::AddEntity {
                    entity,
                    fields: fields_snapshot,
                })
            }

            EditCommand::MovePanel(inner) => {
                let inverse = inner.execute(schedule)?;
                Ok(EditCommand::MovePanel(Box::new(inverse)))
            }

            EditCommand::BatchEdit(cmds) => {
                let mut inverses: Vec<EditCommand> = Vec::with_capacity(cmds.len());
                for cmd in cmds {
                    let inv = cmd.execute(schedule)?;
                    inverses.push(inv);
                }
                inverses.reverse();
                Ok(EditCommand::BatchEdit(inverses))
            }
        }
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn find_registration(
    entity: RuntimeEntityId,
) -> Result<&'static crate::entity::RegisteredEntityType, EditError> {
    registered_entity_types()
        .find(|r| r.type_name == entity.entity_type_name())
        .ok_or(EditError::UnknownEntityType(entity.entity_type_name()))
}

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
    fn push_undo(&mut self, inverse: EditCommand) {
        if self.undo_stack.len() == self.max_depth {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(inverse);
        self.redo_stack.clear();
    }

    /// Pop the most recent undo inverse.
    fn pop_undo(&mut self) -> Option<EditCommand> {
        self.undo_stack.pop_back()
    }

    /// Push a re-executable command onto the redo stack.
    fn push_redo(&mut self, cmd: EditCommand) {
        self.redo_stack.push_back(cmd);
    }

    /// Pop the most recent redo command.
    fn pop_redo(&mut self) -> Option<EditCommand> {
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
    schedule: Schedule,
    history: EditHistory,
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
        Ok(())
    }

    // ── Convenience constructors ──────────────────────────────────────────────

    /// Build an `UpdateField` command that captures the current field value as
    /// `old_value` before writing `new_value`.
    ///
    /// Returns `Err` if the entity is not found or the field is write-only.
    pub fn update_field_cmd(
        &self,
        entity: RuntimeEntityId,
        field: &'static str,
        new_value: FieldValue,
    ) -> Result<EditCommand, EditError> {
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
        let runtime_id = RuntimeEntityId::from_dynamic(panel);
        let time_cmd = self.update_field_cmd(runtime_id, time_field, new_time)?;
        let room_cmd = self.update_field_cmd(runtime_id, room_field, new_room)?;
        Ok(EditCommand::MovePanel(Box::new(EditCommand::BatchEdit(
            vec![time_cmd, room_cmd],
        ))))
    }
}

// ── Helper: snapshot for new entities ────────────────────────────────────────

/// Snapshot the read+write fields of an entity in the schedule.
///
/// Convenience function used externally (e.g. after `build_entity`) to
/// produce the `fields` vector for an [`EditCommand::AddEntity`].
pub fn snapshot_entity(
    schedule: &Schedule,
    entity: impl DynamicEntityId,
) -> Result<Vec<(&'static str, FieldValue)>, EditError> {
    let runtime_id = RuntimeEntityId::from_dynamic(entity);
    let reg = find_registration(runtime_id)?;
    Ok((reg.snapshot_fn)(schedule, runtime_id.entity_uuid()))
}

/// Build an [`EditCommand::AddEntity`] for an entity that has already been
/// inserted into the schedule (e.g. via a builder).
///
/// The command captures a read+write snapshot of the entity so that undo can
/// remove it and redo can recreate it with the same UUID.
pub fn add_entity_cmd(
    schedule: &Schedule,
    entity: impl DynamicEntityId,
) -> Result<EditCommand, EditError> {
    let runtime_id = RuntimeEntityId::from_dynamic(entity);
    let fields = snapshot_entity(schedule, runtime_id)?;
    Ok(EditCommand::AddEntity {
        entity: runtime_id,
        fields,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::build_entity;
    use crate::entity::UuidPreference;
    use crate::field_set::FieldRef;
    use crate::field_value;
    use crate::panel_type::PanelTypeEntityType;
    use crate::schedule::Schedule;

    fn make_panel_type_in_context() -> (EditContext, RuntimeEntityId) {
        let mut sched = Schedule::default();
        let id = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            vec![
                (FieldRef::Name("prefix"), field_value!("GP")),
                (FieldRef::Name("panel_kind"), field_value!("Guest Panel")),
            ],
        )
        .expect("build_entity succeeded");
        let rid = id.into();
        let ctx = EditContext::new(sched);
        (ctx, rid)
    }

    // ── UpdateField ──────────────────────────────────────────────────────────

    #[test]
    fn update_field_applies_and_undoes() {
        let (mut ctx, entity) = make_panel_type_in_context();

        let cmd = ctx
            .update_field_cmd(entity, "prefix", field_value!("AA"))
            .expect("cmd built");
        ctx.apply(cmd).expect("apply succeeded");

        let prefix = ctx
            .schedule()
            .get_internal::<PanelTypeEntityType>(entity.try_into().expect("typed id"))
            .expect("entity present")
            .data
            .prefix
            .clone();
        assert_eq!(prefix, "AA");

        ctx.undo().expect("undo succeeded");

        let prefix_after_undo = ctx
            .schedule()
            .get_internal::<PanelTypeEntityType>(entity.try_into().expect("typed id"))
            .expect("entity present")
            .data
            .prefix
            .clone();
        assert_eq!(prefix_after_undo, "GP");
    }

    #[test]
    fn update_field_redo_reapplies() {
        let (mut ctx, entity) = make_panel_type_in_context();

        let cmd = ctx
            .update_field_cmd(entity, "prefix", field_value!("BB"))
            .expect("cmd built");
        ctx.apply(cmd).expect("apply");
        ctx.undo().expect("undo");
        ctx.redo().expect("redo");

        let prefix = ctx
            .schedule()
            .get_internal::<PanelTypeEntityType>(entity.try_into().expect("typed id"))
            .expect("entity present")
            .data
            .prefix
            .clone();
        assert_eq!(prefix, "BB");
    }

    // ── Undo clears redo stack ───────────────────────────────────────────────

    #[test]
    fn apply_after_undo_clears_redo() {
        let (mut ctx, entity) = make_panel_type_in_context();

        let cmd1 = ctx
            .update_field_cmd(entity, "prefix", field_value!("C1"))
            .unwrap();
        ctx.apply(cmd1).unwrap();
        ctx.undo().unwrap();

        assert_eq!(ctx.history.redo_depth(), 1);

        let cmd2 = ctx
            .update_field_cmd(entity, "prefix", field_value!("C2"))
            .unwrap();
        ctx.apply(cmd2).unwrap();

        assert_eq!(ctx.history.redo_depth(), 0, "redo stack should be cleared");
    }

    // ── AddEntity / RemoveEntity ─────────────────────────────────────────────

    #[test]
    fn add_entity_undo_removes_it() {
        let mut sched = Schedule::default();
        let id = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            vec![
                (FieldRef::Name("prefix"), field_value!("GP")),
                (FieldRef::Name("panel_kind"), field_value!("Guest Panel")),
            ],
        )
        .expect("build_entity");
        let rid: RuntimeEntityId = id.into();
        let add_cmd = add_entity_cmd(&sched, rid).expect("add_entity_cmd");

        let mut ctx = EditContext::new(sched);
        ctx.apply(add_cmd).expect("apply add");
        assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 1);

        ctx.undo().expect("undo add");
        assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 0);
    }

    #[test]
    fn add_entity_undo_then_redo_restores_same_uuid() {
        let mut sched = Schedule::default();
        let id = build_entity::<PanelTypeEntityType>(
            &mut sched,
            UuidPreference::GenerateNew,
            vec![
                (FieldRef::Name("prefix"), field_value!("GP")),
                (FieldRef::Name("panel_kind"), field_value!("Guest Panel")),
            ],
        )
        .expect("build_entity");
        let rid: RuntimeEntityId = id.into();
        let add_cmd = add_entity_cmd(&sched, rid).expect("add_entity_cmd");

        let mut ctx = EditContext::new(sched);
        ctx.apply(add_cmd).expect("apply");
        ctx.undo().expect("undo");
        ctx.redo().expect("redo");

        assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 1);
        let typed = rid.try_into().expect("typed id");
        let data = ctx.schedule().get_internal::<PanelTypeEntityType>(typed);
        assert!(data.is_some(), "entity restored with same UUID");
    }

    #[test]
    fn remove_entity_undo_restores_entity() {
        let (mut ctx, entity) = make_panel_type_in_context();

        let remove_cmd = ctx.remove_entity_cmd(entity).expect("remove_entity_cmd");
        ctx.apply(remove_cmd).expect("apply remove");
        assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 0);

        ctx.undo().expect("undo remove");
        assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 1);

        let typed = entity.try_into().expect("typed id");
        let data = ctx
            .schedule()
            .get_internal::<PanelTypeEntityType>(typed)
            .expect("entity restored");
        assert_eq!(data.data.prefix, "GP");
    }

    // ── BatchEdit ────────────────────────────────────────────────────────────

    #[test]
    fn batch_edit_applies_atomically_and_undoes_in_reverse() {
        let (mut ctx, entity) = make_panel_type_in_context();

        let cmd1 = ctx
            .update_field_cmd(entity, "prefix", field_value!("B1"))
            .unwrap();
        let cmd2 = ctx
            .update_field_cmd(entity, "panel_kind", field_value!("Workshop"))
            .unwrap();
        let batch = EditCommand::BatchEdit(vec![cmd1, cmd2]);
        ctx.apply(batch).expect("apply batch");

        let data = ctx
            .schedule()
            .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
            .unwrap();
        assert_eq!(data.data.prefix, "B1");
        assert_eq!(data.data.panel_kind, "Workshop");

        ctx.undo().expect("undo batch");

        let data_after = ctx
            .schedule()
            .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
            .unwrap();
        assert_eq!(data_after.data.prefix, "GP");
        assert_eq!(data_after.data.panel_kind, "Guest Panel");
    }

    #[test]
    fn batch_edit_redo_reapplies_all() {
        let (mut ctx, entity) = make_panel_type_in_context();

        let cmd1 = ctx
            .update_field_cmd(entity, "prefix", field_value!("C1"))
            .unwrap();
        let cmd2 = ctx
            .update_field_cmd(entity, "panel_kind", field_value!("Concert"))
            .unwrap();
        ctx.apply(EditCommand::BatchEdit(vec![cmd1, cmd2])).unwrap();
        ctx.undo().unwrap();
        ctx.redo().unwrap();

        let data = ctx
            .schedule()
            .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
            .unwrap();
        assert_eq!(data.data.prefix, "C1");
        assert_eq!(data.data.panel_kind, "Concert");
    }

    // ── Dirty state ──────────────────────────────────────────────────────────

    #[test]
    fn dirty_state_tracks_correctly() {
        let (mut ctx, entity) = make_panel_type_in_context();

        assert!(!ctx.is_dirty());

        let cmd = ctx
            .update_field_cmd(entity, "prefix", field_value!("X"))
            .unwrap();
        ctx.apply(cmd).unwrap();
        assert!(ctx.is_dirty());

        ctx.mark_clean();
        assert!(!ctx.is_dirty());

        ctx.undo().unwrap();
        assert!(!ctx.is_dirty());
    }

    // ── History bounds ───────────────────────────────────────────────────────

    #[test]
    fn history_respects_max_depth() {
        let sched = Schedule::default();
        let mut ctx = EditContext::with_history_depth(sched, 3);
        let mut sched2 = Schedule::default();
        for i in 0u8..5 {
            let id = build_entity::<PanelTypeEntityType>(
                &mut sched2,
                UuidPreference::GenerateNew,
                vec![
                    (FieldRef::Name("prefix"), field_value!(format!("P{i}"))),
                    (FieldRef::Name("panel_kind"), field_value!("Kind")),
                ],
            )
            .expect("build");
            let rid: RuntimeEntityId = id.into();
            let add_cmd = add_entity_cmd(&sched2, rid).expect("add cmd");
            let _ = ctx.apply(add_cmd);
        }
        assert_eq!(
            ctx.history.undo_depth(),
            3,
            "undo stack should not exceed max_depth"
        );
    }

    // ── Error cases ─────────────────────────────────────────────────────────

    #[test]
    fn undo_on_empty_stack_returns_error() {
        let sched = Schedule::default();
        let mut ctx = EditContext::new(sched);
        assert!(matches!(ctx.undo(), Err(EditError::NothingToUndo)));
    }

    #[test]
    fn redo_on_empty_stack_returns_error() {
        let sched = Schedule::default();
        let mut ctx = EditContext::new(sched);
        assert!(matches!(ctx.redo(), Err(EditError::NothingToRedo)));
    }
}
