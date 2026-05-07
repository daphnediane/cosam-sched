/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`Schedule`] — top-level coordination container.
//!
//! Holds all entity storage, relationship edges, and schedule metadata.
//! Fully generic: no entity-type imports here; all typed wiring lives in
//! entity modules.

pub mod crdt;
pub mod edge;
pub mod entity;

// Re-exports from submodules
pub use crdt::{LoadError, ScheduleMetadata, FILE_FORMAT_VERSION, FILE_MAGIC};
pub use edge::{
    add_edge, add_edge_helper_field, combine_full_edges, entity_ids_to_field_value,
    field_value_to_entity_ids, field_value_to_runtime_entity_ids,
};
pub use edge::{read_edge, read_full_edge, remove_edge, remove_edge_helper_field, write_edge};

use crate::edge::cache::TransitiveEdgeCache;
use crate::edge::map::RawEdgeMap;
use crate::sidecar::{ChangeState, ScheduleSidecar};
use automerge::AutoCommit;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use uuid::NonNilUuid;

// ── Schedule ──────────────────────────────────────────────────────────────────

/// Top-level schedule container.
///
/// - **Entity storage**: `HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>` —
///   one inner map per entity type; indexed by `TypeId::of::<E::InternalData>()`.
/// - **Edge storage**: a single [`RawEdgeMap`] for all relationships.
/// - **Metadata**: schedule UUID, timestamps, generator info.
///
/// There is no separate `EntityStorage` struct; storage lives directly here.
/// Generic `get_internal` / `insert` dispatch via `TypeId`.
///
/// ## CRDT source of truth
///
/// The authoritative state of every entity lives in the [`AutoCommit`]
/// document `doc`. The `entities` HashMap is a cache that mirrors the
/// document: every successful field write routes through
/// [`crdt::write_field`] before returning, and [`Self::remove_entity`]
/// soft-deletes via the `__deleted` flag. On `load` / `apply_changes` /
/// `merge` the cache is rebuilt in full from the document (FEATURE-022
/// part 2).
///
/// During load the mirror is disabled via [`Self::mirror_enabled`] so that
/// rehydrating entities does not generate redundant writes against the doc.
pub struct Schedule {
    /// Two-level type-erased entity store (cache mirroring the CRDT doc).
    pub(crate) entities: HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>,

    /// Single unified edge store for all entity relationships.
    pub(crate) edges: RawEdgeMap,

    /// Cache for transitive homogeneous-edge relationships (inclusive groups, members, etc.).
    /// Invalidated on any homogeneous-edge mutation; rebuilt lazily on next query.
    pub(crate) transitive_edge_cache: RefCell<Option<TransitiveEdgeCache>>,

    /// Schedule identity and provenance.
    pub metadata: ScheduleMetadata,

    /// Authoritative CRDT document. All non-derived field values flow here
    /// first; `entities` then mirrors the post-write state.
    pub(crate) doc: AutoCommit,

    /// When `false`, field writes skip the CRDT mirror. Used during
    /// bulk rehydration from the document (FEATURE-022 part 2) to avoid
    /// re-generating change records for values already in the doc.
    pub(crate) mirror_enabled: bool,

    /// Ephemeral import-session sidecar (SourceInfo + formula extras).
    /// Never serialized; cleared on `load_from_file`.
    pub(crate) sidecar: ScheduleSidecar,

    /// Per-session change tracking: which entities changed since the last
    /// successful `save_to_file`. Cleared after each save; not persisted.
    pub(crate) change_tracker: HashMap<NonNilUuid, ChangeState>,
}

impl std::fmt::Debug for Schedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schedule")
            .field("entities", &self.entities)
            .field("edges", &self.edges)
            .field("transitive_edge_cache", &self.transitive_edge_cache)
            .field("metadata", &self.metadata)
            .field("mirror_enabled", &self.mirror_enabled)
            .field("change_tracker", &self.change_tracker)
            .finish()
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    // ── Sidecar access ────────────────────────────────────────────────────────

    /// Shared access to the ephemeral import sidecar.
    #[must_use]
    pub fn sidecar(&self) -> &ScheduleSidecar {
        &self.sidecar
    }

    /// Mutable access to the ephemeral import sidecar.
    pub fn sidecar_mut(&mut self) -> &mut ScheduleSidecar {
        &mut self.sidecar
    }

    // ── Change tracking ───────────────────────────────────────────────────────

    /// Return the current [`ChangeState`] for `uuid`.
    ///
    /// Returns [`ChangeState::Unchanged`] for any UUID not in the tracker.
    #[must_use]
    pub fn entity_change_state(&self, uuid: NonNilUuid) -> ChangeState {
        self.change_tracker
            .get(&uuid)
            .copied()
            .unwrap_or(ChangeState::Unchanged)
    }

    /// Update the change state for `uuid`.
    ///
    /// Transitions:
    /// - `Added` is sticky — once added, subsequent `Modified` writes do not
    ///   downgrade the state.
    /// - `Deleted` overrides all states.
    pub(crate) fn mark_entity_changed(&mut self, uuid: NonNilUuid, state: ChangeState) {
        let entry = self.change_tracker.entry(uuid).or_default();
        match (*entry, state) {
            // Deleted always wins.
            (_, ChangeState::Deleted) => *entry = ChangeState::Deleted,
            // Added is sticky; don't downgrade to Modified.
            (ChangeState::Added, ChangeState::Modified) => {}
            // Otherwise apply the new state.
            _ => *entry = state,
        }
    }

    /// Create a new, empty schedule with a fresh v7 UUID.
    #[must_use]
    pub fn new() -> Self {
        let raw = uuid::Uuid::now_v7();
        // SAFETY: Uuid::now_v7() is never nil.
        let schedule_id = unsafe { NonNilUuid::new_unchecked(raw) };
        Self {
            entities: HashMap::new(),
            edges: RawEdgeMap::default(),
            transitive_edge_cache: RefCell::new(None),
            metadata: ScheduleMetadata {
                schedule_id,
                created_at: chrono::Utc::now(),
                modified_at: None,
                generator: String::new(),
                version: 0,
            },
            doc: AutoCommit::new(),
            mirror_enabled: true,
            sidecar: ScheduleSidecar::default(),
            change_tracker: HashMap::new(),
        }
    }
}
