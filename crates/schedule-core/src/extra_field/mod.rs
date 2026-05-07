/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`ExtraFieldDescriptor`] — declared data extras on the path to promotion.
//!
//! The system has three tiers for per-entity columns:
//!
//! 1. **[`crate::field::descriptor::FieldDescriptor`]** — full CRDT field,
//!    entity struct member, editor-exposed (e.g., "AV Notes").
//! 2. **[`ExtraFieldDescriptor`]** (this module) — declared but lightweight;
//!    no struct member; routes to the CRDT `__extra` map.
//! 3. **Unknown extra** — auto-detected at import time; data columns (no
//!    formula) go to the CRDT `__extra` map automatically.
//!
//! When a column earns its own `FieldDescriptor`, remove its
//! `ExtraFieldDescriptor`. The `__extra` CRDT entry becomes unreachable but
//! stays in the doc harmlessly.
//!
//! **Formula columns are not declared here.** They live in the xlsx-layer
//! [`crate::xlsx::columns::FormulaColumnDef`] list and are stored in the
//! sidecar, not the CRDT.

use inventory::collect;

/// A declared-but-lightweight extra data column.
///
/// `ExtraFieldDescriptor` is for **data** extras only — columns whose values
/// are user-editable strings that should survive save/load and be shared
/// between users via the CRDT doc. Formula-bearing columns belong in the xlsx
/// module's `FormulaColumnDef` list instead.
///
/// # Registration
///
/// Declare a static and submit it via `inventory::submit!`:
///
/// ```rust,ignore
/// use schedule_core::extra_field::ExtraFieldDescriptor;
///
/// pub static EXTRA_TECH_NOTES: ExtraFieldDescriptor = ExtraFieldDescriptor {
///     name: "tech_notes",
///     display: "Tech Notes",
///     description: "Technical setup notes for this panel.",
///     aliases: &["Tech Notes", "TechNotes"],
///     entity_type: "panel",
/// };
/// inventory::submit! { &EXTRA_TECH_NOTES }
/// ```
#[derive(Debug)]
pub struct ExtraFieldDescriptor {
    /// Canonical key used in the CRDT `__extra` map.
    pub name: &'static str,
    /// Human-readable label for the editor UI.
    pub display: &'static str,
    /// Short description shown in tool-tips or help text.
    pub description: &'static str,
    /// Alternative column names accepted during XLSX import (case-insensitive).
    pub aliases: &'static [&'static str],
    /// `TYPE_NAME` of the entity type this applies to (e.g. `"panel"`), or
    /// `""` to apply to any entity type.
    pub entity_type: &'static str,
}

collect!(ExtraFieldDescriptor);

/// Look up an `ExtraFieldDescriptor` by canonical name or alias, optionally
/// filtered to a specific entity type.
///
/// Returns the first descriptor whose `name` or any `alias` matches
/// `column_name` (case-insensitive) and whose `entity_type` is either `""` or
/// equal to `type_name`.
#[must_use]
pub fn find_extra_descriptor(
    column_name: &str,
    type_name: &str,
) -> Option<&'static ExtraFieldDescriptor> {
    let col_lower = column_name.to_lowercase();
    inventory::iter::<ExtraFieldDescriptor>
        .into_iter()
        .find(|d| {
            let type_matches = d.entity_type.is_empty() || d.entity_type == type_name;
            if !type_matches {
                return false;
            }
            if d.name.to_lowercase() == col_lower {
                return true;
            }
            d.aliases.iter().any(|a| a.to_lowercase() == col_lower)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_extra_descriptor_no_match() {
        assert!(find_extra_descriptor("nonexistent_column_xyz", "panel").is_none());
    }
}
