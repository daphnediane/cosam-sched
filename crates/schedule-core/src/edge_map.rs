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
//! ## Heterogeneous edges (different entity types, e.g. Panel → Presenter)
//!
//! Both endpoints store the other in `edges`. The relationship is effectively
//! undirected at the storage level; the caller determines direction by which
//! UUID it looks up and what type it filters for.
//!
//! ## Homogeneous edges (same entity type, e.g. Presenter → Presenter groups)
//!
//! Forward edges are stored in `edges` (source → targets). Reverse edges are
//! stored in `homogeneous_reverse` (target → sources). This separation avoids
//! direction ambiguity: without it, a Presenter UUID in `edges` could be both a
//! heterogeneous reverse-entry (panel back-link) and a homogeneous forward-entry
//! (group), with no way to distinguish them if both appeared in the same list.

use crate::entity::RuntimeEntityId;
use std::collections::HashMap;
use uuid::NonNilUuid;

/// Unified bidirectional edge store used by [`crate::schedule::Schedule`].
///
/// `Schedule` wraps/unwraps the raw `NonNilUuid` / `RuntimeEntityId` values
/// into typed [`crate::entity::EntityId`]s via its generic
/// `edges_from` / `edges_to` / `edge_add` / `edge_remove` / `edge_set` methods.
#[derive(Debug, Default, Clone)]
pub struct RawEdgeMap {
    /// For heterogeneous edges: undirected — each endpoint stores the other (with type).
    /// For homogeneous forward edges: source stores its targets.
    edges: HashMap<NonNilUuid, Vec<RuntimeEntityId>>,

    /// Reverse side of homogeneous edges only. Keyed by the right-side (target) UUID;
    /// value is the list of left-side (source) entities.
    homogeneous_reverse: HashMap<NonNilUuid, Vec<RuntimeEntityId>>,
}

impl RawEdgeMap {
    // ── Het edge operations ───────────────────────────────────────────────────

    /// Add a heterogeneous (different-type) edge between `a` and `b`.
    ///
    /// Both endpoints store the other. Idempotent — does nothing if the edge
    /// already exists.
    pub fn add_het(&mut self, a: RuntimeEntityId, b: RuntimeEntityId) {
        if !self
            .edges
            .get(&a.uuid())
            .is_some_and(|v| v.iter().any(|e| e.uuid() == b.uuid()))
        {
            self.edges.entry(a.uuid()).or_default().push(b);
        }
        if !self
            .edges
            .get(&b.uuid())
            .is_some_and(|v| v.iter().any(|e| e.uuid() == a.uuid()))
        {
            self.edges.entry(b.uuid()).or_default().push(a);
        }
    }

    /// Remove a heterogeneous edge between `a_uuid` and `b_uuid`.
    ///
    /// No-op if the edge does not exist.
    pub fn remove_het(&mut self, a_uuid: NonNilUuid, b_uuid: NonNilUuid) {
        if let Some(v) = self.edges.get_mut(&a_uuid) {
            v.retain(|e| e.uuid() != b_uuid);
        }
        if let Some(v) = self.edges.get_mut(&b_uuid) {
            v.retain(|e| e.uuid() != a_uuid);
        }
    }

    // ── Homo edge operations ──────────────────────────────────────────────────

    /// Add a homogeneous (same-type) directed edge: `forward` → `reverse`.
    ///
    /// - `forward`: the source entity (stored in `edges`)
    /// - `reverse`: the target entity (stored in `homogeneous_reverse`)
    ///
    /// Idempotent — does nothing if the edge already exists.
    pub fn add_homo(&mut self, forward: RuntimeEntityId, reverse: RuntimeEntityId) {
        if !self
            .edges
            .get(&forward.uuid())
            .is_some_and(|v| v.iter().any(|e| e.uuid() == reverse.uuid()))
        {
            self.edges.entry(forward.uuid()).or_default().push(reverse);
        }
        if !self
            .homogeneous_reverse
            .get(&reverse.uuid())
            .is_some_and(|v| v.iter().any(|e| e.uuid() == forward.uuid()))
        {
            self.homogeneous_reverse
                .entry(reverse.uuid())
                .or_default()
                .push(forward);
        }
    }

    /// Remove a homogeneous directed edge: `forward_uuid` → `reverse_uuid`.
    ///
    /// No-op if the edge does not exist.
    pub fn remove_homo(&mut self, forward_uuid: NonNilUuid, reverse_uuid: NonNilUuid) {
        if let Some(v) = self.edges.get_mut(&forward_uuid) {
            v.retain(|e| e.uuid() != reverse_uuid);
        }
        if let Some(v) = self.homogeneous_reverse.get_mut(&reverse_uuid) {
            v.retain(|e| e.uuid() != forward_uuid);
        }
    }

    // ── Bulk set ──────────────────────────────────────────────────────────────

    /// Replace all neighbors of `from` that match `target_type` with `new_targets`.
    ///
    /// Removes any existing edges from `from` to entries of `target_type`, then
    /// adds edges for each entry in `new_targets`.
    ///
    /// - `is_homo`: `true` if `from` and targets are the same entity type.
    pub fn set_neighbors(
        &mut self,
        from: RuntimeEntityId,
        new_targets: &[RuntimeEntityId],
        target_type: &'static str,
        is_homo: bool,
    ) {
        // Collect UUIDs of existing neighbors of the given type.
        let old_uuids: Vec<NonNilUuid> = self
            .edges
            .get(&from.uuid())
            .into_iter()
            .flatten()
            .filter(|e| e.type_name() == target_type)
            .map(|e| e.uuid())
            .collect();

        // Remove old edges.
        for old_uuid in old_uuids {
            if is_homo {
                self.remove_homo(from.uuid(), old_uuid);
            } else {
                self.remove_het(from.uuid(), old_uuid);
            }
        }

        // Add new edges.
        for &target in new_targets {
            if is_homo {
                self.add_homo(from, target);
            } else {
                self.add_het(from, target);
            }
        }
    }

    // ── Read ──────────────────────────────────────────────────────────────────

    /// All neighbors of `uuid` in `edges` (heterogeneous + homogeneous forward).
    pub fn neighbors(&self, uuid: NonNilUuid) -> &[RuntimeEntityId] {
        self.edges.get(&uuid).map_or(&[], Vec::as_slice)
    }

    /// All homogeneous reverse neighbors of `uuid` (sources whose homogeneous
    /// forward edge points to `uuid`).
    pub fn homo_reverse(&self, uuid: NonNilUuid) -> &[RuntimeEntityId] {
        self.homogeneous_reverse
            .get(&uuid)
            .map_or(&[], Vec::as_slice)
    }

    // ── Cleanup ───────────────────────────────────────────────────────────────

    /// Remove all edges involving `uuid`, maintaining consistency in both maps.
    ///
    /// For each entry in `edges[uuid]`:
    /// - If neighbor type == `type_name` (homogeneous forward edge): remove `uuid`
    ///   from `homogeneous_reverse[neighbor]`.
    /// - Otherwise (heterogeneous edge): remove `uuid` from `edges[neighbor]`.
    ///
    /// For each entry in `homogeneous_reverse[uuid]`:
    /// - Remove `uuid` from `edges[source]` (homogeneous reverse cleanup).
    ///
    /// Then removes `uuid` from both maps.
    pub fn clear_all(&mut self, uuid: NonNilUuid, type_name: &'static str) {
        // Process edges[uuid] — heterogeneous reverse cleanup + homogeneous reverse-map cleanup.
        if let Some(neighbors) = self.edges.remove(&uuid) {
            for neighbor in neighbors {
                if neighbor.type_name() == type_name {
                    // Homogeneous forward edge: remove uuid from homogeneous_reverse[neighbor]
                    if let Some(v) = self.homogeneous_reverse.get_mut(&neighbor.uuid()) {
                        v.retain(|e| e.uuid() != uuid);
                    }
                } else {
                    // Heterogeneous edge: remove uuid from edges[neighbor]
                    if let Some(v) = self.edges.get_mut(&neighbor.uuid()) {
                        v.retain(|e| e.uuid() != uuid);
                    }
                }
            }
        }

        // Process homogeneous_reverse[uuid] — homogeneous forward (edges[source]) cleanup.
        if let Some(sources) = self.homogeneous_reverse.remove(&uuid) {
            for source in sources {
                if let Some(v) = self.edges.get_mut(&source.uuid()) {
                    v.retain(|e| e.uuid() != uuid);
                }
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;
    use crate::field_set::FieldSet;
    use crate::value::ValidationError;
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

    fn nnu(n: u128) -> NonNilUuid {
        NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
    }

    fn rid_a(n: u128) -> RuntimeEntityId {
        unsafe { RuntimeEntityId::from_uuid(nnu(n), TypeA::TYPE_NAME) }
    }

    fn rid_b(n: u128) -> RuntimeEntityId {
        unsafe { RuntimeEntityId::from_uuid(nnu(n), TypeB::TYPE_NAME) }
    }

    // ── Het edge tests ───────────────────────────────────────────────────────

    #[test]
    fn test_het_add_stores_both_directions() {
        let mut map = RawEdgeMap::default();
        let a = rid_a(1);
        let b = rid_b(2);
        map.add_het(a, b);

        assert_eq!(map.neighbors(a.uuid()), &[b]);
        assert_eq!(map.neighbors(b.uuid()), &[a]);
    }

    #[test]
    fn test_het_add_is_idempotent() {
        let mut map = RawEdgeMap::default();
        let a = rid_a(1);
        let b = rid_b(2);
        map.add_het(a, b);
        map.add_het(a, b);

        assert_eq!(map.neighbors(a.uuid()).len(), 1);
        assert_eq!(map.neighbors(b.uuid()).len(), 1);
    }

    #[test]
    fn test_het_remove_clears_both_directions() {
        let mut map = RawEdgeMap::default();
        let a = rid_a(1);
        let b = rid_b(2);
        map.add_het(a, b);
        map.remove_het(a.uuid(), b.uuid());

        assert!(map.neighbors(a.uuid()).is_empty());
        assert!(map.neighbors(b.uuid()).is_empty());
    }

    #[test]
    fn test_het_remove_noop_when_absent() {
        let mut map = RawEdgeMap::default();
        map.remove_het(nnu(1), nnu(2)); // should not panic
    }

    #[test]
    fn test_het_multiple_neighbors() {
        let mut map = RawEdgeMap::default();
        let a = rid_a(1);
        let b1 = rid_b(2);
        let b2 = rid_b(3);
        map.add_het(a, b1);
        map.add_het(a, b2);

        let neighbors = map.neighbors(a.uuid());
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.iter().any(|r| r.uuid() == b1.uuid()));
        assert!(neighbors.iter().any(|r| r.uuid() == b2.uuid()));
    }

    // ── Homo edge tests ──────────────────────────────────────────────────────

    #[test]
    fn test_homo_add_forward_in_edges_reverse_in_homo_reverse() {
        let mut map = RawEdgeMap::default();
        let member = rid_a(10);
        let group = rid_a(20);
        map.add_homo(member, group);

        // Forward: member → group in edges
        assert_eq!(map.neighbors(member.uuid()), &[group]);
        // Reverse: group's members in homogeneous_reverse
        assert_eq!(map.homo_reverse(group.uuid()), &[member]);
        // group is NOT in edges (heterogeneous side) pointing to member
        assert!(map.neighbors(group.uuid()).is_empty());
    }

    #[test]
    fn test_homo_add_is_idempotent() {
        let mut map = RawEdgeMap::default();
        let member = rid_a(10);
        let group = rid_a(20);
        map.add_homo(member, group);
        map.add_homo(member, group);

        assert_eq!(map.neighbors(member.uuid()).len(), 1);
        assert_eq!(map.homo_reverse(group.uuid()).len(), 1);
    }

    #[test]
    fn test_homo_remove_clears_both_sides() {
        let mut map = RawEdgeMap::default();
        let member = rid_a(10);
        let group = rid_a(20);
        map.add_homo(member, group);
        map.remove_homo(member.uuid(), group.uuid());

        assert!(map.neighbors(member.uuid()).is_empty());
        assert!(map.homo_reverse(group.uuid()).is_empty());
    }

    #[test]
    fn test_homo_remove_noop_when_absent() {
        let mut map = RawEdgeMap::default();
        map.remove_homo(nnu(10), nnu(20)); // should not panic
    }

    // ── Coexistence: heterogeneous + homogeneous on same entity ──────────────

    #[test]
    fn test_het_and_homo_coexist_on_same_uuid() {
        let mut map = RawEdgeMap::default();
        let presenter = rid_a(1);
        let panel = rid_b(2);
        let group = rid_a(3);

        map.add_het(presenter, panel); // heterogeneous: presenter ↔ panel
        map.add_homo(presenter, group); // homogeneous: presenter → group

        let neighbors = map.neighbors(presenter.uuid());
        assert_eq!(neighbors.len(), 2);
        // Filter by type_name to distinguish
        let panels: Vec<_> = neighbors
            .iter()
            .filter(|r| r.type_name() == TypeB::TYPE_NAME)
            .collect();
        let groups: Vec<_> = neighbors
            .iter()
            .filter(|r| r.type_name() == TypeA::TYPE_NAME)
            .collect();
        assert_eq!(panels.len(), 1);
        assert_eq!(groups.len(), 1);
        // group appears in homo_reverse
        assert_eq!(map.homo_reverse(group.uuid()), &[presenter]);
    }

    // ── set_neighbors tests ──────────────────────────────────────────────────

    #[test]
    fn test_set_neighbors_het_replaces_matching_type() {
        let mut map = RawEdgeMap::default();
        let a = rid_a(1);
        let b1 = rid_b(10);
        let b2 = rid_b(11);
        let b3 = rid_b(12);
        map.add_het(a, b1);
        map.add_het(a, b2);

        map.set_neighbors(a, &[b3], TypeB::TYPE_NAME, false);

        let neighbors = map.neighbors(a.uuid());
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].uuid(), b3.uuid());
        // Old neighbors no longer point back
        assert!(map.neighbors(b1.uuid()).is_empty());
        assert!(map.neighbors(b2.uuid()).is_empty());
        assert!(map
            .neighbors(b3.uuid())
            .iter()
            .any(|r| r.uuid() == a.uuid()));
    }

    #[test]
    fn test_set_neighbors_het_preserves_other_types() {
        let mut map = RawEdgeMap::default();
        let a = rid_a(1);
        let b = rid_b(10);
        let a2 = rid_a(2); // same type as a
        map.add_het(a, b);
        map.add_het(a, a2); // heterogeneous edge to another TypeA

        // Replace only TypeB neighbors
        map.set_neighbors(a, &[], TypeB::TYPE_NAME, false);

        let neighbors = map.neighbors(a.uuid());
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].uuid(), a2.uuid());
    }

    #[test]
    fn test_set_neighbors_homo_replaces_matching_type() {
        let mut map = RawEdgeMap::default();
        let member = rid_a(1);
        let g1 = rid_a(10);
        let g2 = rid_a(11);
        map.add_homo(member, g1);

        map.set_neighbors(member, &[g2], TypeA::TYPE_NAME, true);

        assert_eq!(map.neighbors(member.uuid()).len(), 1);
        assert_eq!(map.neighbors(member.uuid())[0].uuid(), g2.uuid());
        assert!(map.homo_reverse(g1.uuid()).is_empty());
        assert_eq!(map.homo_reverse(g2.uuid()), &[member]);
    }

    // ── clear_all tests ──────────────────────────────────────────────────────

    #[test]
    fn test_clear_all_removes_het_edges_from_neighbors() {
        let mut map = RawEdgeMap::default();
        let a = rid_a(1);
        let b = rid_b(2);
        map.add_het(a, b);

        map.clear_all(a.uuid(), TypeA::TYPE_NAME);

        assert!(map.neighbors(a.uuid()).is_empty());
        assert!(map.neighbors(b.uuid()).is_empty());
    }

    #[test]
    fn test_clear_all_removes_homo_forward_from_homo_reverse() {
        let mut map = RawEdgeMap::default();
        let member = rid_a(1);
        let group = rid_a(2);
        map.add_homo(member, group);

        map.clear_all(member.uuid(), TypeA::TYPE_NAME);

        assert!(map.neighbors(member.uuid()).is_empty());
        assert!(map.homo_reverse(group.uuid()).is_empty());
    }

    #[test]
    fn test_clear_all_removes_homo_reverse_from_edges() {
        let mut map = RawEdgeMap::default();
        let member = rid_a(1);
        let group = rid_a(2);
        map.add_homo(member, group);

        map.clear_all(group.uuid(), TypeA::TYPE_NAME);

        assert!(map.homo_reverse(group.uuid()).is_empty());
        assert!(map.neighbors(member.uuid()).is_empty());
    }

    #[test]
    fn test_clear_all_mixed_het_and_homo() {
        let mut map = RawEdgeMap::default();
        let presenter = rid_a(1);
        let panel = rid_b(2);
        let group = rid_a(3);
        map.add_het(presenter, panel);
        map.add_homo(presenter, group);

        map.clear_all(presenter.uuid(), TypeA::TYPE_NAME);

        assert!(map.neighbors(presenter.uuid()).is_empty());
        assert!(map.homo_reverse(presenter.uuid()).is_empty());
        assert!(map.neighbors(panel.uuid()).is_empty());
        assert!(map.homo_reverse(group.uuid()).is_empty());
    }

    #[test]
    fn test_clear_all_noop_when_absent() {
        let mut map = RawEdgeMap::default();
        map.clear_all(nnu(99), "type_a"); // should not panic
    }

    // ── Reverse-index consistency ────────────────────────────────────────────

    #[test]
    fn test_homo_reverse_is_consistent_after_multiple_adds() {
        let mut map = RawEdgeMap::default();
        let m1 = rid_a(1);
        let m2 = rid_a(2);
        let g = rid_a(10);
        map.add_homo(m1, g);
        map.add_homo(m2, g);

        let members = map.homo_reverse(g.uuid());
        assert_eq!(members.len(), 2);
        assert!(members.iter().any(|r| r.uuid() == m1.uuid()));
        assert!(members.iter().any(|r| r.uuid() == m2.uuid()));
    }

    #[test]
    fn test_homo_reverse_empty_after_all_members_removed() {
        let mut map = RawEdgeMap::default();
        let m1 = rid_a(1);
        let m2 = rid_a(2);
        let g = rid_a(10);
        map.add_homo(m1, g);
        map.add_homo(m2, g);
        map.remove_homo(m1.uuid(), g.uuid());
        map.remove_homo(m2.uuid(), g.uuid());

        assert!(map.homo_reverse(g.uuid()).is_empty());
    }
}
