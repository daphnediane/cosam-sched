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
pub use edge::{entity_ids_to_field_value, field_value_to_entity_ids};

use crate::edge::cache::TransitiveEdgeCache;
use crate::edge::map::RawEdgeMap;
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
    ///
    /// Outer key: `TypeId::of::<E::InternalData>()`.
    /// Inner key: entity UUID.
    /// Value: `Box<E::InternalData>`.
    pub(crate) entities: HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>,

    /// Single unified edge store for all entity relationships.
    pub(crate) edges: RawEdgeMap,

    /// Cache for transitive homogeneous-edge relationships (inclusive groups, members, etc.).
    /// Set to `None` whenever a homogeneous edge is modified; rebuilt lazily per-entry on query.
    /// Heterogeneous-edge mutations do not touch this field since the cache contains no heterogeneous data.
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
}

impl std::fmt::Debug for Schedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schedule")
            .field("entities", &self.entities)
            .field("edges", &self.edges)
            .field("transitive_edge_cache", &self.transitive_edge_cache)
            .field("metadata", &self.metadata)
            .field("mirror_enabled", &self.mirror_enabled)
            .finish()
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
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
                generator: String::new(),
                version: 0,
            },
            doc: AutoCommit::new(),
            mirror_enabled: true,
        }
    }
}
