/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`RawEdgeMap`] — unified bidirectional relationship storage.
//!
//! All entity UUIDs are globally unique, so a single map covers every
//! relationship regardless of entity type.
//!
//! ## Structure
//!
//! ```text
//! HashMap<NonNilUuid,                  // outer key: entity UUID
//!     HashMap<FieldRef,         // inner key: which field on that entity
//!         Vec<RuntimeFieldNodeId>>>   // values: (field, uuid) of the other side
//! ```
//!
//! Both directions of every edge are stored symmetrically.  Homogeneous and
//! heterogeneous edges are treated identically — no separate `homogeneous_reverse`
//! map is needed because each endpoint's field reference makes the relationship
//! self-describing.
//!
//! ## Example
//!
//! For a Panel ↔ Presenter edge with `FIELD_PRESENTERS` on Panel and
//! `FIELD_PANELS` on Presenter:
//!
//! ```text
//! map[panel_uuid][FIELD_PRESENTERS] = [(FIELD_PANELS, presenter_uuid), ...]
//! map[presenter_uuid][FIELD_PANELS] = [(FIELD_PRESENTERS, panel_uuid), ...]
//! ```
//!
//! For a Presenter → Groups homogeneous edge with `FIELD_GROUPS` on member and
//! `FIELD_MEMBERS` on group:
//!
//! ```text
//! map[member_uuid][FIELD_GROUPS]  = [(FIELD_MEMBERS, group_uuid), ...]
//! map[group_uuid][FIELD_MEMBERS]  = [(FIELD_GROUPS,  member_uuid), ...]
//! ```

use crate::entity::DynamicEntityId;
use crate::field_node_id::{DynamicFieldNodeId, FieldRef, RuntimeFieldNodeId};
use std::collections::HashMap;
use uuid::NonNilUuid;

/// Unified bidirectional edge store used by [`crate::schedule::Schedule`].
///
/// `Schedule` wraps/unwraps the raw [`NonNilUuid`] values into typed
/// [`crate::entity::EntityId`]s via its generic
/// `edges_from` / `edges_to` / `edge_add` / `edge_remove` / `edge_set` methods.
#[derive(Debug, Default, Clone)]
pub struct RawEdgeMap {
    map: HashMap<NonNilUuid, HashMap<(FieldRef, FieldRef), Vec<NonNilUuid>>>,
}

impl RawEdgeMap {
    // ── Mutations ─────────────────────────────────────────────────────────────

    /// Add a bidirectional edge between `from` and `to`.
    ///
    /// Both endpoints store the other.  Idempotent — does nothing if the edge
    /// already exists in either direction.
    pub fn add_edge(&mut self, from: impl DynamicFieldNodeId, to: impl DynamicFieldNodeId) {
        let from_field = FieldRef(from.field());
        let to_field = FieldRef(to.field());
        let from_key = (from_field, to_field);
        let to_key = (to_field, from_field);

        let from_vec = self
            .map
            .entry(from.entity_uuid())
            .or_default()
            .entry(from_key)
            .or_default();
        if !from_vec.contains(&to.entity_uuid()) {
            from_vec.push(to.entity_uuid());
        }
        let to_vec = self
            .map
            .entry(to.entity_uuid())
            .or_default()
            .entry(to_key)
            .or_default();
        if !to_vec.contains(&from.entity_uuid()) {
            to_vec.push(from.entity_uuid());
        }
    }

    /// Remove the bidirectional edge between `from` and `to`.
    ///
    /// No-op if the edge does not exist.
    pub fn remove_edge(&mut self, from: impl DynamicFieldNodeId, to: impl DynamicFieldNodeId) {
        let from_field = FieldRef(from.field());
        let to_field = FieldRef(to.field());
        let from_key = (from_field, to_field);
        let to_key = (to_field, from_field);

        if let Some(inner) = self.map.get_mut(&from.entity_uuid()) {
            if let Some(v) = inner.get_mut(&from_key) {
                v.retain(|uuid| *uuid != to.entity_uuid());
            }
        }
        if let Some(inner) = self.map.get_mut(&to.entity_uuid()) {
            if let Some(v) = inner.get_mut(&to_key) {
                v.retain(|uuid| *uuid != from.entity_uuid());
            }
        }
    }

    /// Set all neighbors for a specific edge `(from, far)` pair.
    ///
    /// Replaces all neighbors reachable via the `(from_field, far_field)` edge
    /// with `new_targets`, maintaining bidirectional consistency.
    ///
    /// 1. Removes `from` as a reverse entry from every current target of this edge.
    /// 2. Overwrites the edge list for `(from_field, far_field)` with `new_targets`.
    /// 3. Inserts `from` as a reverse entry on each new target (idempotent).
    ///
    /// Returns `(added, removed)` UUIDs reflecting the diff vs the previous
    /// neighbor set. Callers can use this to apply incremental CRDT mirror
    /// updates rather than full list rewrites.
    pub fn set_neighbors(
        &mut self,
        from: impl DynamicFieldNodeId,
        far: FieldRef,
        new_targets: impl IntoIterator<Item = impl DynamicEntityId>,
    ) -> (Vec<NonNilUuid>, Vec<NonNilUuid>) {
        let from_field = FieldRef(from.field());
        let from_uuid = from.entity_uuid();
        let edge_key = (from_field, far);
        let reverse_key = (far, from_field);

        // Collect old targets for this specific edge so we can patch their reverse entries.
        let old_uuids: Vec<NonNilUuid> = self
            .map
            .get(&from_uuid)
            .and_then(|inner| inner.get(&edge_key))
            .cloned()
            .unwrap_or_default();

        // Build new neighbor UUIDs directly from entity IDs
        let new_neighbor_uuids: Vec<NonNilUuid> =
            new_targets.into_iter().map(|t| t.entity_uuid()).collect();

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

        // Remove `from` from each removed target's reverse list.
        for old_uuid in &removed {
            if let Some(inner) = self.map.get_mut(old_uuid) {
                if let Some(v) = inner.get_mut(&reverse_key) {
                    v.retain(|uuid| *uuid != from_uuid);
                }
            }
        }

        // Overwrite the edge list.
        let from_inner = self.map.entry(from_uuid).or_default();
        from_inner.insert(edge_key, new_neighbor_uuids);

        // Insert `from` as a reverse entry on each added target (idempotent).
        for target_uuid in &added {
            let v = self
                .map
                .entry(*target_uuid)
                .or_default()
                .entry(reverse_key)
                .or_default();
            if !v.contains(&from_uuid) {
                v.push(from_uuid);
            }
        }

        (added, removed)
    }

    /// Remove all edges involving `uuid`, maintaining bidirectional consistency.
    ///
    /// For each neighbor of `uuid` in any field, removes `uuid` from that
    /// neighbor's reverse entry.  Then drops `uuid`'s outer map entry.
    pub fn clear_all(&mut self, uuid: NonNilUuid) {
        let Some(inner) = self.map.remove(&uuid) else {
            return;
        };
        for ((src_field, dest_field), neighbor_uuids) in inner {
            for neighbor_uuid in neighbor_uuids {
                let reverse_key = (dest_field, src_field);
                if let Some(neighbor_inner) = self.map.get_mut(&neighbor_uuid) {
                    if let Some(v) = neighbor_inner.get_mut(&reverse_key) {
                        v.retain(|u| *u != uuid);
                    }
                }
            }
        }
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// All neighbors of `near_node` reachable via the edge to `far`.
    ///
    /// Returns an empty vec when no edges exist for this `(near_field, far)` pair.
    #[must_use]
    pub fn neighbors(
        &self,
        near_node: impl DynamicFieldNodeId,
        far: FieldRef,
    ) -> Vec<RuntimeFieldNodeId> {
        let near_field = FieldRef(near_node.field());
        let near_uuid = near_node.entity_uuid();
        let edge_key = (near_field, far);

        let Some(inner) = self.map.get(&near_uuid) else {
            return Vec::new();
        };

        let Some(neighbor_uuids) = inner.get(&edge_key) else {
            return Vec::new();
        };

        neighbor_uuids
            .iter()
            .map(|&uuid| {
                // SAFETY: The stored field (far) is always a valid NamedField
                unsafe { RuntimeFieldNodeId::new_unchecked(uuid, far.0) }
            })
            .collect()
    }

    /// All neighbors of `near_node` across all destination fields.
    ///
    /// Returns neighbors for all edges where `near_node` is the source,
    /// regardless of the destination field.
    ///
    /// **Deprecated**: This method is part of the migration away from non-field-based
    /// edges. Use `neighbors` with a specific far field when possible.
    #[deprecated(note = "Use neighbors() with a specific far field when possible")]
    #[must_use]
    pub fn combined_neighbors(
        &self,
        near_node: impl DynamicFieldNodeId,
    ) -> Vec<RuntimeFieldNodeId> {
        let near_uuid = near_node.entity_uuid();

        let Some(inner) = self.map.get(&near_uuid) else {
            return Vec::new();
        };

        let mut result = Vec::new();
        for ((src_field, dest_field), neighbor_uuids) in inner {
            let node_field = near_node.field();
            // Compare data pointers for field descriptor equality
            let src_ptr = src_field.0 as *const dyn crate::field::NamedField as *const ();
            let node_ptr = node_field as *const dyn crate::field::NamedField as *const ();
            if src_ptr == node_ptr {
                for neighbor_uuid in neighbor_uuids {
                    // SAFETY: The stored field (dest_field) is always a valid NamedField
                    result.push(unsafe {
                        RuntimeFieldNodeId::new_unchecked(*neighbor_uuid, dest_field.0)
                    });
                }
            }
        }
        result
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityId, EntityType};
    use crate::field::{CommonFieldData, FieldDescriptor};
    use crate::field_set::FieldSet;
    use crate::value::{
        CrdtFieldType, EdgeKind, FieldCardinality, FieldType, FieldTypeItem, ValidationError,
    };
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
        read_fn: None,
        write_fn: None,
        verify_fn: None,
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
        read_fn: None,
        write_fn: None,
        verify_fn: None,
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
        read_fn: None,
        write_fn: None,
        verify_fn: None,
    };

    fn fr_a1() -> FieldRef {
        FieldRef(&FIELD_A1)
    }
    fn fr_a2() -> FieldRef {
        FieldRef(&FIELD_A2)
    }
    fn fr_b1() -> FieldRef {
        FieldRef(&FIELD_B1)
    }

    fn fn_a1(n: u128) -> RuntimeFieldNodeId {
        // SAFETY: Test fixtures use matching entity types for their fields.
        unsafe { RuntimeFieldNodeId::new_unchecked(nnu(n), &FIELD_A1) }
    }
    fn fn_a2(n: u128) -> RuntimeFieldNodeId {
        // SAFETY: Test fixtures use matching entity types for their fields.
        unsafe { RuntimeFieldNodeId::new_unchecked(nnu(n), &FIELD_A2) }
    }
    fn fn_b1(n: u128) -> RuntimeFieldNodeId {
        // SAFETY: Test fixtures use matching entity types for their fields.
        unsafe { RuntimeFieldNodeId::new_unchecked(nnu(n), &FIELD_B1) }
    }

    fn id_b(n: u128) -> EntityId<TypeB> {
        // SAFETY: Test fixtures use valid UUIDs for TypeB.
        unsafe { EntityId::new_unchecked(nnu(n)) }
    }

    fn nnu(n: u128) -> NonNilUuid {
        NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
    }

    // ── add_edge / neighbors_for_field ───────────────────────────────────────

    #[test]
    fn test_add_edge_stores_both_directions() {
        let mut map = RawEdgeMap::default();
        // Heterogeneous: TypeA.FIELD_A1 ↔ TypeB.FIELD_B1
        map.add_edge(fn_a1(1), fn_b1(2));

        assert_eq!(map.neighbors(fn_a1(1), fr_b1()), &[fn_b1(2)]);
        assert_eq!(map.neighbors(fn_b1(2), fr_a1()), &[fn_a1(1)]);
    }

    #[test]
    fn test_add_edge_is_idempotent() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.add_edge(fn_a1(1), fn_b1(2));

        assert_eq!(map.neighbors(fn_a1(1), fr_b1()).len(), 1);
        assert_eq!(map.neighbors(fn_b1(2), fr_a1()).len(), 1);
    }

    #[test]
    fn test_add_edge_multiple_neighbors() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(10));
        map.add_edge(fn_a1(1), fn_b1(11));

        let neighbors = map.neighbors(fn_a1(1), fr_b1());
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.contains(&fn_b1(10)));
        assert!(neighbors.contains(&fn_b1(11)));
    }

    // ── Homogeneous edges — same entity type, two different fields ───────────

    #[test]
    fn test_homo_edge_both_directions_in_same_map() {
        let mut map = RawEdgeMap::default();
        // member (FIELD_A1) → group (FIELD_A2): member's a1 points at group's a2
        map.add_edge(fn_a1(10), fn_a2(20));

        // Forward: member's FIELD_A1 contains group reference
        assert_eq!(map.neighbors(fn_a1(10), fr_a2()), &[fn_a2(20)]);
        // Reverse: group's FIELD_A2 contains member reference
        assert_eq!(map.neighbors(fn_a2(20), fr_a1()), &[fn_a1(10)]);
        // FIELD_A2 on member is empty (not involved in this edge)
        assert!(map.neighbors(fn_a2(10), fr_a2()).is_empty());
        // FIELD_A1 on group is empty (not involved in this edge)
        assert!(map.neighbors(fn_a1(20), fr_a1()).is_empty());
    }

    #[test]
    fn test_homo_edge_multiple_members_same_group() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_a2(100));
        map.add_edge(fn_a1(2), fn_a2(100));

        // Group's reverse list contains both members
        let members = map.neighbors(fn_a2(100), fr_a1());
        assert_eq!(members.len(), 2);
        assert!(members.contains(&fn_a1(1)));
        assert!(members.contains(&fn_a1(2)));
    }

    // ── Coexistence: het + homo on same entity UUID ──────────────────────────

    #[test]
    fn test_het_and_homo_coexist_on_same_uuid() {
        let mut map = RawEdgeMap::default();
        // Presenter (FIELD_A1) ↔ Panel (FIELD_B1) — heterogeneous
        map.add_edge(fn_a1(1), fn_b1(2));
        // Presenter (FIELD_A2) → Group (FIELD_A2 on group) — homogeneous
        map.add_edge(fn_a2(1), fn_a2(3));

        // entity 1's FIELD_A1 has the panel
        assert_eq!(map.neighbors(fn_a1(1), fr_b1()), &[fn_b1(2)]);
        // entity 1's FIELD_A2 has the group
        assert_eq!(map.neighbors(fn_a2(1), fr_a2()), &[fn_a2(3)]);
        // panel's FIELD_B1 has the presenter back-link
        assert_eq!(map.neighbors(fn_b1(2), fr_a1()), &[fn_a1(1)]);
        // group's FIELD_A2 has the presenter back-link
        assert_eq!(map.neighbors(fn_a2(3), fr_a2()), &[fn_a2(1)]);
    }

    // ── remove_edge ──────────────────────────────────────────────────────────

    #[test]
    fn test_remove_edge_clears_both_directions() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.remove_edge(fn_a1(1), fn_b1(2));

        assert!(map.neighbors(fn_a1(1), fr_a1()).is_empty());
        assert!(map.neighbors(fn_b1(2), fr_b1()).is_empty());
    }

    #[test]
    fn test_remove_edge_noop_when_absent() {
        let mut map = RawEdgeMap::default();
        map.remove_edge(fn_a1(1), fn_b1(2)); // must not panic
    }

    #[test]
    fn test_remove_edge_leaves_other_edges_intact() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(10));
        map.add_edge(fn_a1(1), fn_b1(11));
        map.remove_edge(fn_a1(1), fn_b1(10));

        let neighbors = map.neighbors(fn_a1(1), fr_b1());
        assert_eq!(neighbors.len(), 1);
        assert!(neighbors.contains(&fn_b1(11)));
        assert!(map.neighbors(fn_b1(10), fr_a1()).is_empty());
        assert_eq!(map.neighbors(fn_b1(11), fr_a1()), &[fn_a1(1)]);
    }

    // ── set_neighbors ─────────────────────────────────────────────────────────

    #[test]
    fn test_set_neighbors_replaces_and_patches_reverse() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(10));
        map.add_edge(fn_a1(1), fn_b1(11));

        map.set_neighbors(fn_a1(1), fr_b1(), vec![id_b(12)]);

        let neighbors = map.neighbors(fn_a1(1), fr_b1());
        assert_eq!(neighbors.len(), 1);
        assert!(neighbors.contains(&fn_b1(12)));
        // old targets no longer point back
        assert!(map.neighbors(fn_b1(10), fr_a1()).is_empty());
        assert!(map.neighbors(fn_b1(11), fr_a1()).is_empty());
        // new target points back
        assert_eq!(map.neighbors(fn_b1(12), fr_a1()), &[fn_a1(1)]);
    }

    #[test]
    fn test_set_neighbors_to_empty_clears_all() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.set_neighbors(fn_a1(1), fr_b1(), vec![] as Vec<EntityId<TypeB>>);

        assert!(map.neighbors(fn_a1(1), fr_b1()).is_empty());
        assert!(map.neighbors(fn_b1(2), fr_a1()).is_empty());
    }

    #[test]
    fn test_set_neighbors_preserves_other_fields() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(10));
        map.add_edge(fn_a1(1), fn_a2(20));

        map.set_neighbors(fn_a1(1), fr_b1(), vec![id_b(11)]);

        let neighbors_b = map.neighbors(fn_a1(1), fr_b1());
        assert_eq!(neighbors_b.len(), 1);
        assert!(neighbors_b.contains(&fn_b1(11)));
        // a2 edge preserved
        let neighbors_a = map.neighbors(fn_a1(1), fr_a2());
        assert_eq!(neighbors_a.len(), 1);
        assert!(neighbors_a.contains(&fn_a2(20)));
    }

    // ── clear_all ────────────────────────────────────────────────────────────

    #[test]
    fn test_clear_all_removes_het_edges_from_neighbors() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.clear_all(nnu(1));

        assert!(map.neighbors(fn_a1(1), fr_a1()).is_empty());
        assert!(map.neighbors(fn_b1(2), fr_b1()).is_empty());
    }

    #[test]
    fn test_clear_all_removes_homo_edges_from_both_directions() {
        let mut map = RawEdgeMap::default();
        // member → group
        map.add_edge(fn_a1(10), fn_a2(20));
        map.clear_all(nnu(10));

        assert!(map.neighbors(fn_a1(10), fr_a1()).is_empty());
        // group's reverse entry cleaned up
        assert!(map.neighbors(fn_a2(20), fr_a2()).is_empty());
    }

    #[test]
    fn test_clear_all_target_side_cleans_up_source() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(10), fn_a2(20));
        map.clear_all(nnu(20));

        assert!(map.neighbors(fn_a2(20), fr_a2()).is_empty());
        // member's forward entry cleaned up
        assert!(map.neighbors(fn_a1(10), fr_a1()).is_empty());
    }

    #[test]
    fn test_clear_all_mixed_fields() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.add_edge(fn_a2(1), fn_a2(3));
        map.clear_all(nnu(1));

        assert!(map.neighbors(fn_a1(1), fr_a1()).is_empty());
        assert!(map.neighbors(fn_a2(1), fr_a2()).is_empty());
        assert!(map.neighbors(fn_b1(2), fr_b1()).is_empty());
        assert!(map.neighbors(fn_a2(3), fr_a2()).is_empty());
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
        assert!(map.neighbors(fn_a1(1), fr_a1()).is_empty());
    }

    #[test]
    fn test_neighbors_for_field_wrong_field_on_known_uuid() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        // Query FIELD_A2 on entity 1, which has only FIELD_A1 edges
        assert!(map.neighbors(fn_a2(1), fr_a2()).is_empty());
    }

    // ── Reverse-index consistency ────────────────────────────────────────────

    #[test]
    fn test_reverse_consistent_after_multiple_adds() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_a2(100));
        map.add_edge(fn_a1(2), fn_a2(100));
        map.add_edge(fn_a1(3), fn_a2(100));

        let members = map.neighbors(fn_a2(100), fr_a1());
        assert_eq!(members.len(), 3);
        assert!(members.contains(&fn_a1(1)));
        assert!(members.contains(&fn_a1(2)));
        assert!(members.contains(&fn_a1(3)));
    }

    #[test]
    fn test_reverse_empty_after_all_members_removed() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_a2(100));
        map.add_edge(fn_a1(2), fn_a2(100));
        map.remove_edge(fn_a1(1), fn_a2(100));
        map.remove_edge(fn_a1(2), fn_a2(100));

        assert!(map.neighbors(fn_a2(100), fr_a1()).is_empty());
    }
}
