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
//! HashMap<NonNilUuid,          // outer key: entity UUID
//!     HashMap<FieldId,         // inner key: which field on that entity
//!         Vec<FieldNodeId>>>   // values: (field, uuid) of the other side
//! ```
//!
//! Both directions of every edge are stored symmetrically.  Homogeneous and
//! heterogeneous edges are treated identically — no separate `homogeneous_reverse`
//! map is needed because each endpoint's [`FieldId`] makes the relationship
//! self-describing.
//!
//! ## Example
//!
//! For a Panel ↔ Presenter edge with `FIELD_PRESENTERS` on Panel and
//! `FIELD_PANELS` on Presenter:
//!
//! ```text
//! map[panel_uuid][FIELD_PRESENTERS_id] = [(FIELD_PANELS_id, presenter_uuid), ...]
//! map[presenter_uuid][FIELD_PANELS_id] = [(FIELD_PRESENTERS_id, panel_uuid), ...]
//! ```
//!
//! For a Presenter → Groups homogeneous edge with `FIELD_GROUPS` on member and
//! `FIELD_MEMBERS` on group:
//!
//! ```text
//! map[member_uuid][FIELD_GROUPS_id]  = [(FIELD_MEMBERS_id, group_uuid), ...]
//! map[group_uuid][FIELD_MEMBERS_id]  = [(FIELD_GROUPS_id,  member_uuid), ...]
//! ```

use crate::field_node_id::{FieldId, FieldNodeId};
use std::collections::HashMap;
use uuid::NonNilUuid;

/// Unified bidirectional edge store used by [`crate::schedule::Schedule`].
///
/// `Schedule` wraps/unwraps the raw [`FieldNodeId`] values into typed
/// [`crate::entity::EntityId`]s via its generic
/// `edges_from` / `edges_to` / `edge_add` / `edge_remove` / `edge_set` methods.
#[derive(Debug, Default, Clone)]
pub struct RawEdgeMap {
    map: HashMap<NonNilUuid, HashMap<FieldId, Vec<FieldNodeId>>>,
}

impl RawEdgeMap {
    // ── Mutations ─────────────────────────────────────────────────────────────

    /// Add a bidirectional edge between `from` and `to`.
    ///
    /// Both endpoints store the other.  Idempotent — does nothing if the edge
    /// already exists in either direction.
    pub fn add_edge(&mut self, from: FieldNodeId, to: FieldNodeId) {
        let from_vec = self
            .map
            .entry(from.entity)
            .or_default()
            .entry(from.field)
            .or_default();
        if !from_vec.contains(&to) {
            from_vec.push(to);
        }
        let to_vec = self
            .map
            .entry(to.entity)
            .or_default()
            .entry(to.field)
            .or_default();
        if !to_vec.contains(&from) {
            to_vec.push(from);
        }
    }

    /// Remove the bidirectional edge between `from` and `to`.
    ///
    /// No-op if the edge does not exist.
    pub fn remove_edge(&mut self, from: FieldNodeId, to: FieldNodeId) {
        if let Some(inner) = self.map.get_mut(&from.entity) {
            if let Some(v) = inner.get_mut(&from.field) {
                v.retain(|n| *n != to);
            }
        }
        if let Some(inner) = self.map.get_mut(&to.entity) {
            if let Some(v) = inner.get_mut(&to.field) {
                v.retain(|n| *n != from);
            }
        }
    }

    /// Replace all neighbors in `from`'s field with `new_targets` (bulk set).
    ///
    /// 1. Removes `from` as a reverse entry from every current target.
    /// 2. Overwrites `from`'s field list with `new_targets`.
    /// 3. Inserts `from` as a reverse entry on every new target (idempotent).
    pub fn set_field_neighbors(&mut self, from: FieldNodeId, new_targets: Vec<FieldNodeId>) {
        // Collect old targets so we can patch their reverse entries.
        let old_targets: Vec<FieldNodeId> = self
            .map
            .get(&from.entity)
            .and_then(|inner| inner.get(&from.field))
            .cloned()
            .unwrap_or_default();

        // Remove `from` from each old target's reverse list.
        for old in &old_targets {
            if let Some(inner) = self.map.get_mut(&old.entity) {
                if let Some(v) = inner.get_mut(&old.field) {
                    v.retain(|n| *n != from);
                }
            }
        }

        // Overwrite the from-side field list.
        self.map
            .entry(from.entity)
            .or_default()
            .insert(from.field, new_targets.clone());

        // Insert `from` as a reverse entry on each new target (idempotent).
        for target in &new_targets {
            let v = self
                .map
                .entry(target.entity)
                .or_default()
                .entry(target.field)
                .or_default();
            if !v.contains(&from) {
                v.push(from);
            }
        }
    }

    /// Remove all edges involving `uuid`, maintaining bidirectional consistency.
    ///
    /// For each neighbor of `uuid` in any field, removes `uuid` from that
    /// neighbor's reverse entry.  Then drops `uuid`'s outer map entry.
    pub fn clear_all(&mut self, uuid: NonNilUuid) {
        let Some(inner) = self.map.remove(&uuid) else {
            return;
        };
        for (_, neighbors) in inner {
            for neighbor in neighbors {
                if let Some(neighbor_inner) = self.map.get_mut(&neighbor.entity) {
                    if let Some(v) = neighbor_inner.get_mut(&neighbor.field) {
                        v.retain(|n| n.entity != uuid);
                    }
                }
            }
        }
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// All neighbors of `uuid` reachable via the given `field`.
    ///
    /// Returns an empty slice when no edges exist for this `(uuid, field)` pair.
    #[must_use]
    pub fn neighbors_for_field(&self, uuid: NonNilUuid, field: FieldId) -> &[FieldNodeId] {
        self.map
            .get(&uuid)
            .and_then(|inner| inner.get(&field))
            .map_or(&[], Vec::as_slice)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;
    use crate::field::{FieldDescriptor, FieldDescriptorAny};
    use crate::field_set::FieldSet;
    use crate::value::{
        CrdtFieldType, FieldCardinality, FieldType, FieldTypeItem, ValidationError,
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
        name: "a1",
        display: "A1",
        description: "Test field A1",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
        example: "",
        order: 0,
        read_fn: None,
        write_fn: None,
        verify_fn: None,
    };

    static FIELD_A2: FieldDescriptor<TypeA> = FieldDescriptor {
        name: "a2",
        display: "A2",
        description: "Test field A2",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
        example: "",
        order: 1,
        read_fn: None,
        write_fn: None,
        verify_fn: None,
    };

    static FIELD_B1: FieldDescriptor<TypeB> = FieldDescriptor {
        name: "b1",
        display: "B1",
        description: "Test field B1",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
        example: "",
        order: 0,
        read_fn: None,
        write_fn: None,
        verify_fn: None,
    };

    fn fid_a1() -> FieldId {
        FieldId::of::<TypeA>(&FIELD_A1)
    }
    fn fid_a2() -> FieldId {
        FieldId::of::<TypeA>(&FIELD_A2)
    }
    fn fid_b1() -> FieldId {
        FieldId::of::<TypeB>(&FIELD_B1)
    }

    fn fn_a1(n: u128) -> FieldNodeId {
        FieldNodeId::new(fid_a1(), nnu(n))
    }
    fn fn_a2(n: u128) -> FieldNodeId {
        FieldNodeId::new(fid_a2(), nnu(n))
    }
    fn fn_b1(n: u128) -> FieldNodeId {
        FieldNodeId::new(fid_b1(), nnu(n))
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

        assert_eq!(map.neighbors_for_field(nnu(1), fid_a1()), &[fn_b1(2)]);
        assert_eq!(map.neighbors_for_field(nnu(2), fid_b1()), &[fn_a1(1)]);
    }

    #[test]
    fn test_add_edge_is_idempotent() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.add_edge(fn_a1(1), fn_b1(2));

        assert_eq!(map.neighbors_for_field(nnu(1), fid_a1()).len(), 1);
        assert_eq!(map.neighbors_for_field(nnu(2), fid_b1()).len(), 1);
    }

    #[test]
    fn test_add_edge_multiple_neighbors() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(10));
        map.add_edge(fn_a1(1), fn_b1(11));

        let neighbors = map.neighbors_for_field(nnu(1), fid_a1());
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
        assert_eq!(map.neighbors_for_field(nnu(10), fid_a1()), &[fn_a2(20)]);
        // Reverse: group's FIELD_A2 contains member reference
        assert_eq!(map.neighbors_for_field(nnu(20), fid_a2()), &[fn_a1(10)]);
        // FIELD_A2 on member is empty (not involved in this edge)
        assert!(map.neighbors_for_field(nnu(10), fid_a2()).is_empty());
        // FIELD_A1 on group is empty (not involved in this edge)
        assert!(map.neighbors_for_field(nnu(20), fid_a1()).is_empty());
    }

    #[test]
    fn test_homo_edge_multiple_members_same_group() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_a2(100));
        map.add_edge(fn_a1(2), fn_a2(100));

        // Group's reverse list contains both members
        let members = map.neighbors_for_field(nnu(100), fid_a2());
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
        assert_eq!(map.neighbors_for_field(nnu(1), fid_a1()), &[fn_b1(2)]);
        // entity 1's FIELD_A2 has the group
        assert_eq!(map.neighbors_for_field(nnu(1), fid_a2()), &[fn_a2(3)]);
        // panel's FIELD_B1 has the presenter back-link
        assert_eq!(map.neighbors_for_field(nnu(2), fid_b1()), &[fn_a1(1)]);
        // group's FIELD_A2 has the presenter back-link
        assert_eq!(map.neighbors_for_field(nnu(3), fid_a2()), &[fn_a2(1)]);
    }

    // ── remove_edge ──────────────────────────────────────────────────────────

    #[test]
    fn test_remove_edge_clears_both_directions() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.remove_edge(fn_a1(1), fn_b1(2));

        assert!(map.neighbors_for_field(nnu(1), fid_a1()).is_empty());
        assert!(map.neighbors_for_field(nnu(2), fid_b1()).is_empty());
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

        let neighbors = map.neighbors_for_field(nnu(1), fid_a1());
        assert_eq!(neighbors.len(), 1);
        assert!(neighbors.contains(&fn_b1(11)));
        assert!(map.neighbors_for_field(nnu(10), fid_b1()).is_empty());
        assert_eq!(map.neighbors_for_field(nnu(11), fid_b1()), &[fn_a1(1)]);
    }

    // ── set_field_neighbors ──────────────────────────────────────────────────

    #[test]
    fn test_set_field_neighbors_replaces_and_patches_reverse() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(10));
        map.add_edge(fn_a1(1), fn_b1(11));

        map.set_field_neighbors(fn_a1(1), vec![fn_b1(12)]);

        let neighbors = map.neighbors_for_field(nnu(1), fid_a1());
        assert_eq!(neighbors.len(), 1);
        assert!(neighbors.contains(&fn_b1(12)));
        // old targets no longer point back
        assert!(map.neighbors_for_field(nnu(10), fid_b1()).is_empty());
        assert!(map.neighbors_for_field(nnu(11), fid_b1()).is_empty());
        // new target points back
        assert_eq!(map.neighbors_for_field(nnu(12), fid_b1()), &[fn_a1(1)]);
    }

    #[test]
    fn test_set_field_neighbors_to_empty_clears_all() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(10));
        map.set_field_neighbors(fn_a1(1), vec![]);

        assert!(map.neighbors_for_field(nnu(1), fid_a1()).is_empty());
        assert!(map.neighbors_for_field(nnu(10), fid_b1()).is_empty());
    }

    #[test]
    fn test_set_field_neighbors_preserves_other_fields() {
        let mut map = RawEdgeMap::default();
        // entity 1 has FIELD_A1 edge to b1(10) and FIELD_A2 edge to a2(20)
        map.add_edge(fn_a1(1), fn_b1(10));
        map.add_edge(fn_a2(1), fn_a2(20));

        // Only replace the FIELD_A1 neighbors
        map.set_field_neighbors(fn_a1(1), vec![fn_b1(11)]);

        assert_eq!(map.neighbors_for_field(nnu(1), fid_a1()), &[fn_b1(11)]);
        // FIELD_A2 neighbors unchanged
        assert_eq!(map.neighbors_for_field(nnu(1), fid_a2()), &[fn_a2(20)]);
    }

    // ── clear_all ────────────────────────────────────────────────────────────

    #[test]
    fn test_clear_all_removes_het_edges_from_neighbors() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.clear_all(nnu(1));

        assert!(map.neighbors_for_field(nnu(1), fid_a1()).is_empty());
        assert!(map.neighbors_for_field(nnu(2), fid_b1()).is_empty());
    }

    #[test]
    fn test_clear_all_removes_homo_edges_from_both_directions() {
        let mut map = RawEdgeMap::default();
        // member → group
        map.add_edge(fn_a1(10), fn_a2(20));
        map.clear_all(nnu(10));

        assert!(map.neighbors_for_field(nnu(10), fid_a1()).is_empty());
        // group's reverse entry cleaned up
        assert!(map.neighbors_for_field(nnu(20), fid_a2()).is_empty());
    }

    #[test]
    fn test_clear_all_target_side_cleans_up_source() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(10), fn_a2(20));
        map.clear_all(nnu(20));

        assert!(map.neighbors_for_field(nnu(20), fid_a2()).is_empty());
        // member's forward entry cleaned up
        assert!(map.neighbors_for_field(nnu(10), fid_a1()).is_empty());
    }

    #[test]
    fn test_clear_all_mixed_fields() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        map.add_edge(fn_a2(1), fn_a2(3));
        map.clear_all(nnu(1));

        assert!(map.neighbors_for_field(nnu(1), fid_a1()).is_empty());
        assert!(map.neighbors_for_field(nnu(1), fid_a2()).is_empty());
        assert!(map.neighbors_for_field(nnu(2), fid_b1()).is_empty());
        assert!(map.neighbors_for_field(nnu(3), fid_a2()).is_empty());
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
        assert!(map.neighbors_for_field(nnu(1), fid_a1()).is_empty());
    }

    #[test]
    fn test_neighbors_for_field_wrong_field_on_known_uuid() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_b1(2));
        // Query FIELD_A2 on entity 1, which has only FIELD_A1 edges
        assert!(map.neighbors_for_field(nnu(1), fid_a2()).is_empty());
    }

    // ── Reverse-index consistency ────────────────────────────────────────────

    #[test]
    fn test_reverse_consistent_after_multiple_adds() {
        let mut map = RawEdgeMap::default();
        map.add_edge(fn_a1(1), fn_a2(100));
        map.add_edge(fn_a1(2), fn_a2(100));
        map.add_edge(fn_a1(3), fn_a2(100));

        let members = map.neighbors_for_field(nnu(100), fid_a2());
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

        assert!(map.neighbors_for_field(nnu(100), fid_a2()).is_empty());
    }
}
