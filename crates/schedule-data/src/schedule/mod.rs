/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule container — minimal stub for Phase 2 macro testing.
//!
//! Full implementation is tracked in FEATURE-008.

use crate::entity::{EntityType, PresenterId};
use uuid::NonNilUuid;

/// Central schedule container.
///
/// This is a stub implementation sufficient for computed-field closures
/// to compile.  Full storage and query support is added in FEATURE-008.
#[derive(Debug, Default)]
pub struct Schedule;

impl Schedule {
    pub fn new() -> Self {
        Self
    }

    // -----------------------------------------------------------------------
    // Presenter relationship queries (stubs — FEATURE-008 fills these in)
    // -----------------------------------------------------------------------

    /// Return the group IDs that `presenter_id` belongs to.
    pub fn get_presenter_groups(&self, _presenter_id: PresenterId) -> Vec<PresenterId> {
        vec![]
    }

    /// Return the member IDs of `presenter_id` (when it is a group).
    pub fn get_presenter_members(&self, _presenter_id: PresenterId) -> Vec<PresenterId> {
        vec![]
    }

    // -----------------------------------------------------------------------
    // Generic name lookup helper used by computed-field closures
    // -----------------------------------------------------------------------

    /// Return display names for a slice of UUIDs belonging to entity type `T`.
    pub fn get_entity_names<T: EntityType>(&self, _uuids: &[NonNilUuid]) -> Vec<String> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_stub_default() {
        let s = Schedule;
        assert!(s
            .get_presenter_groups(unsafe {
                use uuid::Uuid;
                PresenterId::from_uuid(uuid::NonNilUuid::new_unchecked(Uuid::from_bytes([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                ])))
            })
            .is_empty());
    }
}
