/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edit command types for schedule mutations with undo/redo support.
//!
//! Every schedule mutation flows through an [`EditCommand`].  Each variant
//! carries enough state to both *apply* and *undo* itself.  Commands are
//! intentionally value-typed (no references) so they can live on undo/redo
//! stacks without lifetime concerns.

use crate::entity::EntityKind;
use crate::field::{FieldError, FieldValue};
use crate::schedule::Schedule;
use uuid::NonNilUuid;

/// A single, reversible schedule mutation.
///
/// `EditCommand` is the atom of the edit system.  The [`Compound`] variant
/// bundles multiple atomic commands into one user-visible undo/redo step
/// (e.g., "add tagged presenter to panel" may create a presenter, a group,
/// membership edges, and a panel-presenter link).
#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    /// Update a single field value on an existing entity.
    ///
    /// The command stores both old and new values so that `apply` writes
    /// `new_value` and `undo` restores `old_value`.
    UpdateField {
        /// Which entity type the target belongs to.
        kind: EntityKind,
        /// The entity's UUID.
        uuid: NonNilUuid,
        /// Canonical field name (as returned by `NamedField::name`).
        field_name: String,
        /// Value before the edit (captured at command creation time).
        old_value: FieldValue,
        /// Value to write when applying the command.
        new_value: FieldValue,
    },

    /// A bundle of commands that form a single undo/redo step.
    ///
    /// `apply` runs all sub-commands in order; `undo` reverses them.
    /// Compound commands may be nested, though one level is typical.
    Compound {
        /// Human-readable label for UI display (e.g., "Add presenter to panel").
        label: String,
        /// Sub-commands executed in order on apply, reversed on undo.
        commands: Vec<EditCommand>,
    },
}

impl EditCommand {
    // ------------------------------------------------------------------
    // Constructors
    // ------------------------------------------------------------------

    /// Create an `UpdateField` command by reading the current value from the
    /// schedule as `old_value`.
    ///
    /// Returns `Err` if the entity or field cannot be found.
    pub fn update_field(
        schedule: &Schedule,
        kind: EntityKind,
        uuid: NonNilUuid,
        field_name: &str,
        new_value: FieldValue,
    ) -> Result<Self, FieldError> {
        let old_value = schedule
            .read_field_value(kind, uuid, field_name)?
            .unwrap_or(FieldValue::None);
        Ok(EditCommand::UpdateField {
            kind,
            uuid,
            field_name: field_name.to_string(),
            old_value,
            new_value,
        })
    }

    /// Create a compound command from a label and a list of sub-commands.
    pub fn compound(label: impl Into<String>, commands: Vec<EditCommand>) -> Self {
        EditCommand::Compound {
            label: label.into(),
            commands,
        }
    }

    // ------------------------------------------------------------------
    // Apply / Undo
    // ------------------------------------------------------------------

    /// Apply this command to the schedule (forward direction).
    pub fn apply(&self, schedule: &mut Schedule) -> Result<(), FieldError> {
        match self {
            EditCommand::UpdateField {
                kind,
                uuid,
                field_name,
                new_value,
                ..
            } => schedule.write_field_value(*kind, *uuid, field_name, new_value.clone()),

            EditCommand::Compound { commands, .. } => {
                for (i, cmd) in commands.iter().enumerate() {
                    if let Err(e) = cmd.apply(schedule) {
                        // Roll back already-applied sub-commands on failure
                        for prev in commands[..i].iter().rev() {
                            // Best-effort undo; ignore errors during rollback
                            let _ = prev.undo(schedule);
                        }
                        return Err(e);
                    }
                }
                Ok(())
            }
        }
    }

    /// Undo this command (reverse direction).
    pub fn undo(&self, schedule: &mut Schedule) -> Result<(), FieldError> {
        match self {
            EditCommand::UpdateField {
                kind,
                uuid,
                field_name,
                old_value,
                ..
            } => schedule.write_field_value(*kind, *uuid, field_name, old_value.clone()),

            EditCommand::Compound { commands, .. } => {
                for cmd in commands.iter().rev() {
                    cmd.undo(schedule)?;
                }
                Ok(())
            }
        }
    }

    /// Human-readable summary of this command for debugging / UI.
    pub fn description(&self) -> String {
        match self {
            EditCommand::UpdateField {
                kind, field_name, ..
            } => format!("Update {:?}.{}", kind, field_name),
            EditCommand::Compound { label, commands } => {
                format!("{} ({} sub-commands)", label, commands.len())
            }
        }
    }
}
