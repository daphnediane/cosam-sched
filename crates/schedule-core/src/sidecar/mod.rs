/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Ephemeral per-session sidecar data for [`Schedule`].
//!
//! The sidecar holds two kinds of data that are intentionally kept outside the
//! CRDT document:
//!
//! - **[`SourceInfo`]** — where each entity was originally imported from (file,
//!   sheet, row). Populated during XLSX import; used by `update_xlsx` to locate
//!   the correct spreadsheet row for each entity. Lost on save/load — callers
//!   must re-import to repopulate.
//!
//! - **[`SidecarFormulaField`]** — formula-bearing cells from XLSX columns that
//!   are spreadsheet-internal (e.g. `Lstart`, `Lend`). Preserved so that
//!   `update_xlsx` can write them back without overwriting user formulas.
//!
//! The sidecar is intentionally never serialized — it is an import-session
//! artifact that does not belong in the long-lived schedule file. See
//! [`crate::schedule::crdt`] for the file format which omits it.
//!
//! ## Change tracking
//!
//! [`ChangeState`] is also defined here because it tracks per-session mutation
//! state with the same ephemeral lifetime as the sidecar.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::NonNilUuid;

// ── SourceInfo ─────────────────────────────────────────────────────────────────

/// The XLSX source location where an entity was originally imported from.
#[derive(Debug, Clone)]
pub struct XlsxSourceInfo {
    /// Path to the file this entity was imported from, if known.
    pub file_path: Option<String>,
    /// Sheet name within the workbook.
    pub sheet_name: String,
    /// 1-based row index of this entity in the sheet (header row is excluded).
    pub row_index: u32,
    /// Timestamp of this import operation.
    pub import_time: DateTime<Utc>,
}

/// Origin of an entity — where it was created.
#[derive(Debug, Clone)]
pub enum EntityOrigin {
    /// Imported from an XLSX spreadsheet.
    Xlsx(XlsxSourceInfo),
    /// Created directly in the editor.
    Editor {
        /// When the entity was created.
        at: DateTime<Utc>,
    },
}

// ── Formula fields ─────────────────────────────────────────────────────────────

/// A preserved formula-column cell from XLSX, stored in the sidecar for
/// round-trip fidelity through `update_xlsx`.
#[derive(Debug, Clone)]
pub struct SidecarFormulaField {
    /// The formula string (e.g. `"=[@Start Time]+[@Duration]"`), if the cell
    /// was actually a formula cell. `None` if the cell held a plain value.
    pub formula: Option<String>,
    /// The evaluated display value at the time of import (for reference only).
    pub display_value: String,
}

// ── EntitySidecar ─────────────────────────────────────────────────────────────

/// All ephemeral sidecar data associated with one entity UUID.
#[derive(Debug, Clone, Default)]
pub struct EntitySidecar {
    /// Where this entity was created / imported from.
    pub origin: Option<EntityOrigin>,
    /// Preserved formula-column cells keyed by column name.
    pub formula_extras: HashMap<String, SidecarFormulaField>,
    /// Original XLSX sort key `(column_index, row_index)` recorded at import
    /// time for [`crate::tables::presenter::PresenterEntityType`] entities.
    /// Used to normalize `sort_index` after all presenters are imported.
    pub xlsx_sort_key: Option<(u32, u32)>,
}

// ── ScheduleSidecar ───────────────────────────────────────────────────────────

/// Session-scoped sidecar store, keyed by entity UUID.
///
/// Intentionally not `Serialize`/`Deserialize` — this data is never written
/// to the `.cosam` file.
#[derive(Debug, Default)]
pub struct ScheduleSidecar {
    entries: HashMap<NonNilUuid, EntitySidecar>,
}

impl ScheduleSidecar {
    /// Return a shared reference to the sidecar entry for `uuid`, if any.
    #[must_use]
    pub fn get(&self, uuid: NonNilUuid) -> Option<&EntitySidecar> {
        self.entries.get(&uuid)
    }

    /// Return a mutable reference to the sidecar entry for `uuid`, creating a
    /// default entry if none exists.
    pub fn get_or_insert(&mut self, uuid: NonNilUuid) -> &mut EntitySidecar {
        self.entries.entry(uuid).or_default()
    }

    /// Set the origin for `uuid`, creating the entry if needed.
    pub fn set_origin(&mut self, uuid: NonNilUuid, origin: EntityOrigin) {
        self.get_or_insert(uuid).origin = Some(origin);
    }

    /// Store a formula-column cell for `uuid`.
    pub fn set_formula_extra(
        &mut self,
        uuid: NonNilUuid,
        column_name: impl Into<String>,
        field: SidecarFormulaField,
    ) {
        self.get_or_insert(uuid)
            .formula_extras
            .insert(column_name.into(), field);
    }

    /// Return the formula extra for `(uuid, column_name)`, if present.
    #[must_use]
    pub fn get_formula_extra(
        &self,
        uuid: NonNilUuid,
        column_name: &str,
    ) -> Option<&SidecarFormulaField> {
        self.entries.get(&uuid)?.formula_extras.get(column_name)
    }

    /// Remove all entries (called after a save or when reloading from file).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Iterate over all `(uuid, sidecar)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (NonNilUuid, &EntitySidecar)> {
        self.entries.iter().map(|(&uuid, sidecar)| (uuid, sidecar))
    }
}

// ── ChangeState ────────────────────────────────────────────────────────────────

/// How an entity has changed relative to the last successful save.
///
/// Tracked in memory only; reset to [`Unchanged`][ChangeState::Unchanged] for
/// all entities after each `save_to_file` call. Used by `update_xlsx` to
/// decide which spreadsheet rows to patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChangeState {
    /// Entity was created since the last save (or this is a freshly imported schedule).
    Added,
    /// One or more fields were modified since the last save.
    Modified,
    /// Entity was soft-deleted since the last save.
    Deleted,
    /// No changes since the last save (or since import).
    #[default]
    Unchanged,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uuid() -> NonNilUuid {
        let u = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
        NonNilUuid::new(u).unwrap()
    }

    #[test]
    fn test_sidecar_set_and_get_origin() {
        let mut sidecar = ScheduleSidecar::default();
        let uuid = test_uuid();
        sidecar.set_origin(
            uuid,
            EntityOrigin::Xlsx(XlsxSourceInfo {
                file_path: Some("test.xlsx".into()),
                sheet_name: "Schedule".into(),
                row_index: 5,
                import_time: chrono::Utc::now(),
            }),
        );
        let entry = sidecar.get(uuid).expect("entry should exist");
        assert!(matches!(entry.origin, Some(EntityOrigin::Xlsx(_))));
    }

    #[test]
    fn test_sidecar_clear() {
        let mut sidecar = ScheduleSidecar::default();
        let uuid = test_uuid();
        sidecar.set_origin(
            uuid,
            EntityOrigin::Editor {
                at: chrono::Utc::now(),
            },
        );
        sidecar.clear();
        assert!(sidecar.get(uuid).is_none());
    }

    #[test]
    fn test_sidecar_formula_extra() {
        let mut sidecar = ScheduleSidecar::default();
        let uuid = test_uuid();
        sidecar.set_formula_extra(
            uuid,
            "Lstart",
            SidecarFormulaField {
                formula: Some("=IF(1,2,3)".into()),
                display_value: "10:00".into(),
            },
        );
        let field = sidecar.get_formula_extra(uuid, "Lstart").unwrap();
        assert_eq!(field.formula.as_deref(), Some("=IF(1,2,3)"));
    }

    #[test]
    fn test_change_state_default() {
        assert_eq!(ChangeState::default(), ChangeState::Unchanged);
    }
}
