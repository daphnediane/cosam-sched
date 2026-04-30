/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`EditCommand`] enum and execution logic.

use crate::edit::builder::BuildError;
use crate::entity::{
    registered_entity_types, DynamicEntityId, EntityTyped, EntityUuid, RuntimeEntityId,
};
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue};
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
/// Construct commands via [`crate::edit::EditContext`] helper methods rather than directly,
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
    MovePanel(Box<EditCommand>),

    /// Execute a sequence of commands as a single atomic undo/redo unit.
    BatchEdit(Vec<EditCommand>),
}

impl EditCommand {
    /// Apply this command to the given schedule, returning its inverse.
    pub fn execute(self, schedule: &mut Schedule) -> Result<EditCommand, EditError> {
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

pub(crate) fn find_registration(
    entity: RuntimeEntityId,
) -> Result<&'static crate::entity::RegisteredEntityType, EditError> {
    registered_entity_types()
        .find(|r| r.type_name == entity.entity_type_name())
        .ok_or(EditError::UnknownEntityType(entity.entity_type_name()))
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
