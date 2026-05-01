/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge map storage and operations.
//!
//! HashMap<NonNilUuid,                 // outer key: entity UUID
//!     HashMap<FieldId,                // inner key: which field on that entity
//!         Vec<RuntimeEntityId>>>      // values: entity UUIDs of connected entities
//!
//! Both directions of every edge are stored symmetrically.  Homogeneous and
//! heterogeneous edges are treated identically - no separate "homogeneous_reverse"
//! map is needed because each endpoint field reference makes the relationship
//! self-describing.
//!
//! ## Example
//!
//! For a Panel <-> Presenter edge with FIELD_PRESENTERS on Panel and
//! FIELD_PANELS on Presenter:
//!
//! ```text,no_run
//! map[panel_uuid][FIELD_PRESENTERS] = [(FIELD_PANELS, presenter_uuid), ...]
//! map[presenter_uuid][FIELD_PANELS] = [(FIELD_PRESENTERS, panel_uuid), ...]
//! ```
//!
//! For a Presenter -> Groups homogeneous edge with FIELD_GROUPS on member and
//! FIELD_MEMBERS on group:
//!
//! ```text,no_run
//! map[member_uuid][FIELD_GROUPS]  = [(FIELD_MEMBERS, group_uuid), ...]
//! map[group_uuid][FIELD_MEMBERS]  = [(FIELD_GROUPS,  member_uuid), ...]
//! ```

use crate::edge::id::FullEdge;
use crate::entity::{DynamicEntityId, RuntimeEntityId};
use std::collections::HashMap;
use thiserror::Error;
use uuid::NonNilUuid;

// ── EdgeError ─────────────────────────────────────────────────────────────────

/// Errors produced by edge operations.
#[derive(Debug, Error)]
pub enum EdgeError {
    /// Entity type mismatch for the near endpoint of an edge.
    #[error("entity type mismatch: near entity is {actual} but edge.near expects {expected}")]
    NearTypeMismatch {
        actual: &'static str,
        expected: &'static str,
    },

    /// Entity type mismatch for the far endpoint of an edge.
    #[error("entity type mismatch: far entity is {actual} but edge.far expects {expected}")]
    FarTypeMismatch {
        actual: &'static str,
        expected: &'static str,
    },
}

/// Unified bidirectional edge store used by [`crate::schedule::Schedule`].
///
/// `Schedule` wraps/unwraps the raw [`NonNilUuid`] values into typed
/// [`crate::entity::EntityId`]s via its generic
/// `edges_from` / `edges_to` / `edge_add` / `edge_remove` / `edge_set` methods.
#[derive(Debug, Default, Clone)]
pub struct RawEdgeMap {
    map: HashMap<NonNilUuid, HashMap<FullEdge, Vec<NonNilUuid>>>,
}

impl RawEdgeMap {
    // ── Mutations ─────────────────────────────────────────────────────────────

    /// Add a bidirectional edge between `from` and `to` using the given edge.
    ///
    /// Both endpoints store the other. Idempotent — does nothing if the edge
    /// already exists in either direction.
    ///
    /// The edge specifies which field on each entity stores the relationship.
    ///
    /// # Errors
    /// Returns `EdgeError::NearTypeMismatch` if `from.entity_type_name()` != `edge.near.entity_type_name()`.
    /// Returns `EdgeError::FarTypeMismatch` if `to.entity_type_name()` != `edge.far.entity_type_name()`.
    pub fn add_edge(
        &mut self,
        from: impl DynamicEntityId,
        to: impl DynamicEntityId,
        edge: FullEdge,
    ) -> Result<(), EdgeError> {
        // Validate entity types match the edge's expected types
        if from.entity_type_name() != edge.near.entity_type_name() {
            return Err(EdgeError::NearTypeMismatch {
                actual: from.entity_type_name(),
                expected: edge.near.entity_type_name(),
            });
        }
        if to.entity_type_name() != edge.far.entity_type_name() {
            return Err(EdgeError::FarTypeMismatch {
                actual: to.entity_type_name(),
                expected: edge.far.entity_type_name(),
            });
        }

        let from_uuid = from.entity_uuid();
        let to_uuid = to.entity_uuid();

        let from_vec = self
            .map
            .entry(from_uuid)
            .or_default()
            .entry(edge)
            .or_default();
        if !from_vec.contains(&to_uuid) {
            from_vec.push(to_uuid);
        }
        let to_vec = self
            .map
            .entry(to_uuid)
            .or_default()
            .entry(edge.flip())
            .or_default();
        if !to_vec.contains(&from_uuid) {
            to_vec.push(from_uuid);
        }
        Ok(())
    }

    /// Remove the bidirectional edge between `from` and `to` using the given edge.
    ///
    /// No-op if the edge does not exist.
    ///
    /// The edge specifies which field on each entity stores the relationship.
    pub fn remove_edge(
        &mut self,
        from: impl DynamicEntityId,
        to: impl DynamicEntityId,
        edge: FullEdge,
    ) {
        let from_uuid = from.entity_uuid();
        let to_uuid = to.entity_uuid();

        if let Some(inner) = self.map.get_mut(&from_uuid) {
            if let Some(v) = inner.get_mut(&edge) {
                v.retain(|uuid| *uuid != to_uuid);
            }
        }
        if let Some(inner) = self.map.get_mut(&to_uuid) {
            if let Some(v) = inner.get_mut(&edge.flip()) {
                v.retain(|uuid| *uuid != from_uuid);
            }
        }
    }

    /// Set all neighbors for a specific edge.
    ///
    /// Replaces all neighbors reachable via the given edge from `near`
    /// with `new_targets`, maintaining bidirectional consistency.
    ///
    /// 1. Removes `near` as a reverse entry from every current target of this edge.
    /// 2. Overwrites the edge list with `new_targets`.
    /// 3. Inserts `near` as a reverse entry on each new target (idempotent).
    ///
    /// Returns `(added, removed)` UUIDs reflecting the diff vs the previous
    /// neighbor set. Callers can use this to apply incremental CRDT mirror
    /// updates rather than full list rewrites.
    ///
    /// # Errors
    /// Returns `EdgeError::NearTypeMismatch` if `near.entity_type_name()` != `edge.near.entity_type_name()`.
    /// Returns `EdgeError::FarTypeMismatch` if any target in `new_targets` has `entity_type_name()` != `edge.far.entity_type_name()`.
    pub fn set_neighbors(
        &mut self,
        near: impl DynamicEntityId,
        edge: FullEdge,
        new_targets: impl IntoIterator<Item = impl DynamicEntityId>,
    ) -> Result<(Vec<NonNilUuid>, Vec<NonNilUuid>), EdgeError> {
        // Validate entity types match the edge's expected types
        if near.entity_type_name() != edge.near.entity_type_name() {
            return Err(EdgeError::NearTypeMismatch {
                actual: near.entity_type_name(),
                expected: edge.near.entity_type_name(),
            });
        }

        let near_uuid = near.entity_uuid();
        let reverse_edge = edge.flip();

        // Collect old targets for this specific edge so we can patch their reverse entries.
        let old_uuids: Vec<NonNilUuid> = self
            .map
            .get(&near_uuid)
            .and_then(|inner| inner.get(&edge))
            .cloned()
            .unwrap_or_default();

        // Build new neighbor UUIDs and validate target types
        let new_neighbor_uuids: Vec<NonNilUuid> = new_targets
            .into_iter()
            .map(|t| {
                if t.entity_type_name() != edge.far.entity_type_name() {
                    return Err(EdgeError::FarTypeMismatch {
                        actual: t.entity_type_name(),
                        expected: edge.far.entity_type_name(),
                    });
                }
                Ok(t.entity_uuid())
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Compute diffs.
        let added: Vec<NonNilUuid> = new_neighbor_uuids
            .iter()
            .filter(|u| !old_uuids.contains(u))
            .copied()
            .collect();
        let removed: Vec<NonNilUuid> = old_uuids
            .iter()
            .filter(|u| !new_neighbor_uuids.contains(u))
            .copied()
            .collect();

        // Remove `near` from each removed target's reverse list.
        for old_uuid in &removed {
            if let Some(inner) = self.map.get_mut(old_uuid) {
                if let Some(v) = inner.get_mut(&reverse_edge) {
                    v.retain(|uuid| *uuid != near_uuid);
                }
            }
        }

        // Overwrite the edge list.
        let near_inner = self.map.entry(near_uuid).or_default();
        near_inner.insert(edge, new_neighbor_uuids);

        // Insert `near` as a reverse entry on each added target (idempotent).
        for target_uuid in &added {
            let v = self
                .map
                .entry(*target_uuid)
                .or_default()
                .entry(reverse_edge)
                .or_default();
            if !v.contains(&near_uuid) {
                v.push(near_uuid);
            }
        }

        Ok((added, removed))
    }

    /// Remove all edges involving `uuid`, maintaining bidirectional consistency.
    ///
    /// For each neighbor of `uuid` in any field, removes `uuid` from that
    /// neighbor's reverse entry.  Then drops `uuid`'s outer map entry.
    pub fn clear_all(&mut self, uuid: NonNilUuid) {
        let Some(inner) = self.map.remove(&uuid) else {
            return;
        };
        for (edge, neighbor_uuids) in inner {
            for neighbor_uuid in neighbor_uuids {
                let reverse_edge = edge.flip();
                if let Some(neighbor_inner) = self.map.get_mut(&neighbor_uuid) {
                    if let Some(v) = neighbor_inner.get_mut(&reverse_edge) {
                        v.retain(|u| *u != uuid);
                    }
                }
            }
        }
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// All neighbors of `near` reachable via the given edge.
    ///
    /// Returns an empty vec when no edges exist for this edge.
    #[must_use]
    pub fn neighbors(&self, near: impl DynamicEntityId, edge: FullEdge) -> Vec<RuntimeEntityId> {
        let near_uuid = near.entity_uuid();

        let Some(inner) = self.map.get(&near_uuid) else {
            return Vec::new();
        };

        let Some(neighbor_uuids) = inner.get(&edge) else {
            return Vec::new();
        };

        neighbor_uuids
            .iter()
            .map(|&uuid| {
                // SAFETY: The stored field (far) is always a valid HalfEdge with a valid entity type
                unsafe { RuntimeEntityId::new_unchecked(uuid, edge.far.entity_type_name()) }
            })
            .collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::CrdtFieldType;
    use crate::edge::EdgeKind;
    use crate::entity::{EntityType, RuntimeEntityId};
    use crate::field::set::FieldSet;
    use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor};
    use crate::value::{FieldCardinality, FieldType, FieldTypeItem, ValidationError};
    use uuid::{NonNilUuid, Uuid};

    // ── Minimal mock entity types ────────────────────────────────────────────

    struct TypeA;
    struct TypeB;

    #[derive(Clone, Debug)]
    struct MockData;

    impl EntityType for TypeA {
        type InternalData = MockData;
        type Data = MockData;
        const TYPE_NAME: &'static str = "type_a";
        fn uuid_namespace() -> &'static Uuid {
            static NS: std::sync::LazyLock<Uuid> =
                std::sync::LazyLock::new(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"type_a"));
            &NS
        }
        fn field_set() -> &'static FieldSet<Self> {
            unimplemented!()
        }
        fn export(_: &MockData) -> MockData {
            MockData
        }
        fn validate(_: &MockData) -> Vec<ValidationError> {
            vec![]
        }
    }

    impl EntityType for TypeB {
        type InternalData = MockData;
        type Data = MockData;
        const TYPE_NAME: &'static str = "type_b";
        fn uuid_namespace() -> &'static Uuid {
            static NS: std::sync::LazyLock<Uuid> =
                std::sync::LazyLock::new(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"type_b"));
            &NS
        }
        fn field_set() -> &'static FieldSet<Self> {
            unimplemented!()
        }
        fn export(_: &MockData) -> MockData {
            MockData
        }
        fn validate(_: &MockData) -> Vec<ValidationError> {
            vec![]
        }
    }

    // ── Static field descriptors for two TypeA fields and one TypeB field ────

    static FIELD_A1: FieldDescriptor<TypeA> = FieldDescriptor {
        data: CommonFieldData {
            name: "a1",
            display: "A1",
            description: "Test field A1",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
            example: "",
            order: 0,
        },
        required: false,
        edge_kind: EdgeKind::NonEdge,
        crdt_type: CrdtFieldType::Derived,
        cb: FieldCallbacks {
            read_fn: None,
            write_fn: None,
            verify_fn: None,
        },
    };

    static FIELD_A2: FieldDescriptor<TypeA> = FieldDescriptor {
        data: CommonFieldData {
            name: "a2",
            display: "A2",
            description: "Test field A2",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
            example: "",
            order: 1,
        },
        required: false,
        edge_kind: EdgeKind::NonEdge,
        crdt_type: CrdtFieldType::Derived,
        cb: FieldCallbacks {
            read_fn: None,
            write_fn: None,
            verify_fn: None,
        },
    };

    static FIELD_B1: FieldDescriptor<TypeB> = FieldDescriptor {
        data: CommonFieldData {
            name: "b1",
            display: "B1",
            description: "Test field B1",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
            example: "",
            order: 0,
        },
        required: false,
        edge_kind: EdgeKind::NonEdge,
        crdt_type: CrdtFieldType::Derived,
        cb: FieldCallbacks {
            read_fn: None,
            write_fn: None,
            verify_fn: None,
        },
    };

    fn fn_a1(n: u128) -> RuntimeEntityId {
        // SAFETY: Test fixtures use matching entity types for their fields.
        unsafe { RuntimeEntityId::new_unchecked(nnu(n), TypeA::TYPE_NAME) }
    }
    fn fn_a2(n: u128) -> RuntimeEntityId {
        // SAFETY: Test fixtures use matching entity types for their fields.
        unsafe { RuntimeEntityId::new_unchecked(nnu(n), TypeA::TYPE_NAME) }
    }
    fn fn_b1(n: u128) -> RuntimeEntityId {
        // SAFETY: Test fixtures use matching entity types for their fields.
        unsafe { RuntimeEntityId::new_unchecked(nnu(n), TypeB::TYPE_NAME) }
    }

    fn nnu(n: u128) -> NonNilUuid {
        NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
    }

    // ── add_edge / neighbors_for_field ───────────────────────────────────────
    #[test]
    fn test_add_edge_stores_both_directions() {
        let mut map = RawEdgeMap::default();
        // Heterogeneous: TypeA.FIELD_A1 ↔ TypeB.FIELD_B1
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();

        let neighbors_a = map.neighbors(fn_a1(1), edge_ab);
        assert_eq!(neighbors_a[0], fn_b1(2));
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        let neighbors_b = map.neighbors(fn_b1(2), edge_ba);
        assert_eq!(neighbors_b[0], fn_a1(1));
    }

    #[test]
    fn test_add_edge_is_idempotent() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();

        assert_eq!(map.neighbors(fn_a1(1), edge_ab).len(), 1);
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        assert_eq!(map.neighbors(fn_b1(2), edge_ba).len(), 1);
    }

    #[test]
    fn test_add_edge_multiple_neighbors() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(10), edge_ab).unwrap();
        map.add_edge(fn_a1(1), fn_b1(11), edge_ab).unwrap();

        let neighbors = map.neighbors(fn_a1(1), edge_ab);
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&fn_b1(10)));
        assert!(neighbors.contains(&fn_b1(11)));
    }

    // ── Homogeneous edges — same entity type, two different fields ───────────

    #[test]
    fn test_homo_edge_both_directions_in_same_map() {
        let mut map = RawEdgeMap::default();
        // member (FIELD_A1) → group (FIELD_A2): member's a1 points at group's a2
        let edge = FIELD_A1.edge_to(&FIELD_A2);
        map.add_edge(fn_a1(10), fn_a2(20), edge).unwrap();

        // Forward: member's FIELD_A1 contains group reference
        assert_eq!(map.neighbors(fn_a1(10), edge), &[fn_a2(20)]);
        let edge_reverse = FIELD_A2.edge_to(&FIELD_A1);
        // Reverse: group's FIELD_A2 contains member reference
        assert_eq!(map.neighbors(fn_a2(20), edge_reverse), &[fn_a1(10)]);
        // FIELD_A2 on member is empty (member is on near side, not far side)
        assert!(map.neighbors(fn_a1(10), edge_reverse).is_empty());
        // FIELD_A1 on group is empty (group is on far side, not near side)
        assert!(map.neighbors(fn_a2(20), edge).is_empty());
    }

    #[test]
    fn test_homo_edge_multiple_members_same_group() {
        let mut map = RawEdgeMap::default();
        let edge = FIELD_A1.edge_to(&FIELD_A2);
        map.add_edge(fn_a1(1), fn_a2(100), edge).unwrap();
        map.add_edge(fn_a1(2), fn_a2(100), edge).unwrap();

        // Group's reverse list contains both members
        let edge_reverse = FIELD_A2.edge_to(&FIELD_A1);
        let members = map.neighbors(fn_a2(100), edge_reverse);
        assert_eq!(members.len(), 2);
        assert!(members.contains(&fn_a1(1)));
        assert!(members.contains(&fn_a1(2)));
    }

    // ── Coexistence: het + homo on same entity UUID ──────────────────────────

    #[test]
    fn test_het_and_homo_coexist_on_same_uuid() {
        let mut map = RawEdgeMap::default();
        // Presenter (FIELD_A1) ↔ Panel (FIELD_B1) — heterogeneous
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();
        // Presenter (FIELD_A2) → Group (FIELD_A2 on group) — homogeneous
        let edge_aa = FIELD_A2.edge_to(&FIELD_A2);
        map.add_edge(fn_a2(1), fn_a2(3), edge_aa).unwrap();

        // entity 1's FIELD_A1 has the panel
        assert_eq!(map.neighbors(fn_a1(1), edge_ab), &[fn_b1(2)]);
        // entity 1's FIELD_A2 has the group
        assert_eq!(map.neighbors(fn_a2(1), edge_aa), &[fn_a2(3)]);
        // panel's FIELD_B1 has the presenter back-link
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        assert_eq!(map.neighbors(fn_b1(2), edge_ba), &[fn_a1(1)]);
        // group's FIELD_A2 has the presenter back-link
        assert_eq!(map.neighbors(fn_a2(3), edge_aa), &[fn_a2(1)]);
    }

    // ── remove_edge ──────────────────────────────────────────────────────────

    #[test]
    fn test_remove_edge_clears_both_directions() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();
        map.remove_edge(fn_a1(1), fn_b1(2), edge_ab);

        assert!(map.neighbors(fn_a1(1), edge_ab).is_empty());
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_b1(2), edge_ba).is_empty());
    }

    #[test]
    fn test_remove_edge_noop_when_absent() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.remove_edge(fn_a1(1), fn_b1(2), edge_ab); // must not panic
    }

    #[test]
    fn test_remove_edge_leaves_other_edges_intact() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(10), edge_ab).unwrap();
        map.add_edge(fn_a1(1), fn_b1(11), edge_ab).unwrap();
        map.remove_edge(fn_a1(1), fn_b1(10), edge_ab);

        let neighbors = map.neighbors(fn_a1(1), edge_ab);
        assert_eq!(neighbors.len(), 1);
        assert!(neighbors.contains(&fn_b1(11)));
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_b1(10), edge_ba).is_empty());
        assert_eq!(map.neighbors(fn_b1(11), edge_ba), &[fn_a1(1)]);
    }

    // ── set_neighbors ─────────────────────────────────────────────────────────
    // TODO: Update set_neighbors tests to work with new API
    #[ignore]
    #[test]
    fn test_set_neighbors_replaces_and_patches_reverse() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(10), edge_ab).unwrap();
        map.add_edge(fn_a1(1), fn_b1(11), edge_ab).unwrap();

        map.set_neighbors(fn_a1(1), edge_ab, vec![fn_b1(12)])
            .unwrap();

        let neighbors = map.neighbors(fn_a1(1), edge_ab);
        assert_eq!(neighbors.len(), 1);
        assert!(neighbors.contains(&fn_b1(12)));
        // old targets no longer point back
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_b1(10), edge_ba).is_empty());
        assert!(map.neighbors(fn_b1(11), edge_ba).is_empty());
        // new target points back
        assert_eq!(map.neighbors(fn_b1(12), edge_ba), &[fn_a1(1)]);
    }

    #[ignore]
    #[test]
    fn test_set_neighbors_to_empty_clears_all() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();
        map.set_neighbors(fn_a1(1), edge_ab, vec![] as Vec<RuntimeEntityId>)
            .unwrap();

        assert!(map.neighbors(fn_a1(1), edge_ab).is_empty());
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_b1(2), edge_ba).is_empty());
    }

    #[ignore]
    #[test]
    fn test_set_neighbors_preserves_other_fields() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(10), edge_ab).unwrap();
        let edge_aa = FIELD_A2.edge_to(&FIELD_A2);
        map.add_edge(fn_a1(1), fn_a2(20), edge_aa).unwrap();

        map.set_neighbors(fn_a1(1), edge_ab, vec![fn_b1(11)])
            .unwrap();

        let neighbors_b = map.neighbors(fn_a1(1), edge_ab);
        assert_eq!(neighbors_b.len(), 1);
        assert!(neighbors_b.contains(&fn_b1(11)));
        // a2 edge preserved
        let neighbors_a = map.neighbors(fn_a1(1), edge_aa);
        assert_eq!(neighbors_a.len(), 1);
        assert!(neighbors_a.contains(&fn_a2(20)));
    }

    // ── clear_all ────────────────────────────────────────────────────────────

    #[test]
    fn test_clear_all_removes_het_edges_from_neighbors() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();
        map.clear_all(nnu(1));

        assert!(map.neighbors(fn_a1(1), edge_ab).is_empty());
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_b1(2), edge_ba).is_empty());
    }

    #[test]
    fn test_clear_all_removes_homo_edges_from_both_directions() {
        let mut map = RawEdgeMap::default();
        // member → group
        let edge = FIELD_A1.edge_to(&FIELD_A2);
        map.add_edge(fn_a1(10), fn_a2(20), edge).unwrap();
        map.clear_all(nnu(10));

        assert!(map.neighbors(fn_a1(10), edge).is_empty());
        // group's reverse entry cleaned up
        let edge_reverse = FIELD_A2.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_a2(20), edge_reverse).is_empty());
    }

    #[test]
    fn test_clear_all_target_side_cleans_up_source() {
        let mut map = RawEdgeMap::default();
        let edge = FIELD_A1.edge_to(&FIELD_A2);
        map.add_edge(fn_a1(10), fn_a2(20), edge).unwrap();
        map.clear_all(nnu(20));

        assert!(map.neighbors(fn_a2(20), edge).is_empty());
        // member's forward entry cleaned up
        let edge_reverse = FIELD_A2.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_a1(10), edge_reverse).is_empty());
    }

    #[test]
    fn test_clear_all_mixed_fields() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();
        let edge_aa = FIELD_A2.edge_to(&FIELD_A2);
        map.add_edge(fn_a1(1), fn_a2(3), edge_aa).unwrap();
        map.clear_all(nnu(1));

        assert!(map.neighbors(fn_a1(1), edge_ab).is_empty());
        assert!(map.neighbors(fn_a2(1), edge_aa).is_empty());
        let edge_ba = FIELD_B1.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_b1(2), edge_ba).is_empty());
        assert!(map.neighbors(fn_a2(3), edge_aa).is_empty());
    }

    #[test]
    fn test_clear_all_noop_when_absent() {
        let mut map = RawEdgeMap::default();
        map.clear_all(nnu(99)); // must not panic
    }

    // ── neighbors_for_field returns empty for unknown uuid/field ─────────────

    #[test]
    fn test_neighbors_for_field_unknown_uuid() {
        let map = RawEdgeMap::default();
        let edge = FIELD_A1.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_a1(1), edge).is_empty());
    }

    #[test]
    fn test_neighbors_for_field_wrong_field_on_known_uuid() {
        let mut map = RawEdgeMap::default();
        let edge_ab = FIELD_A1.edge_to(&FIELD_B1);
        map.add_edge(fn_a1(1), fn_b1(2), edge_ab).unwrap();
        // Query FIELD_A2 on entity 1, which has only FIELD_A1 edges
        let edge_aa = FIELD_A2.edge_to(&FIELD_A2);
        assert!(map.neighbors(fn_a2(1), edge_aa).is_empty());
    }

    // ── Reverse-index consistency ────────────────────────────────────────────

    #[test]
    fn test_reverse_consistent_after_multiple_adds() {
        let mut map = RawEdgeMap::default();
        let edge = FIELD_A1.edge_to(&FIELD_A2);
        map.add_edge(fn_a1(1), fn_a2(100), edge).unwrap();
        map.add_edge(fn_a1(2), fn_a2(100), edge).unwrap();
        map.add_edge(fn_a1(3), fn_a2(100), edge).unwrap();

        let edge_reverse = FIELD_A2.edge_to(&FIELD_A1);
        let members = map.neighbors(fn_a2(100), edge_reverse);
        assert_eq!(members.len(), 3);
        assert!(members.contains(&fn_a1(1)));
        assert!(members.contains(&fn_a1(2)));
        assert!(members.contains(&fn_a1(3)));
    }

    #[test]
    fn test_reverse_empty_after_all_members_removed() {
        let mut map = RawEdgeMap::default();
        let edge = FIELD_A1.edge_to(&FIELD_A2);
        map.add_edge(fn_a1(1), fn_a2(100), edge).unwrap();
        map.add_edge(fn_a1(2), fn_a2(100), edge).unwrap();
        map.remove_edge(fn_a1(1), fn_a2(100), edge);
        map.remove_edge(fn_a1(2), fn_a2(100), edge);

        let edge_reverse = FIELD_A2.edge_to(&FIELD_A1);
        assert!(map.neighbors(fn_a2(100), edge_reverse).is_empty());
    }
}
