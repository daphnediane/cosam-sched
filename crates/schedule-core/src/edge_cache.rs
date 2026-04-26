/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`HomoEdgeCache`] — transitive homogeneous-edge relationship cache.
//!
//! Caches transitive closures of homogeneous-edge relationships (same entity
//! type on both ends) to enable efficient queries like "all groups a presenter
//! belongs to" or "all members of a group including nested groups".
//!
//! The cache is stored as `Option<HomoEdgeCache>` on [`crate::schedule::Schedule`]
//! via a `RefCell`. Setting it to `None` invalidates the cache; it is rebuilt
//! lazily per-entry on the next query.
//!
//! ## What is NOT cached here
//!
//! Heterogeneous-edge transitive queries (e.g. Panel → Inclusive Presenters)
//! require multi-type traversal and are implemented as field read functions in
//! the entity modules ([`crate::panel`], [`crate::presenter`]), composed from
//! `inclusive_edges_from` / `inclusive_edges_to` calls on the schedule.

use crate::edge_map::RawEdgeMap;
use crate::field_node_id::FieldId;
use std::collections::{HashMap, HashSet};
use uuid::NonNilUuid;

// ── HomoEdgeCache ─────────────────────────────────────────────────────────────────

/// Cached transitive homogeneous-edge relationships.
///
/// Only the computed transitive maps are stored here; direct edges are always
/// queried live from [`RawEdgeMap`]. Entries are populated lazily and remain
/// valid until the cache is set to `None` (invalidated) by any transitive-edge
/// mutation.
///
/// Two traversal directions are cached independently:
///
/// - **`inclusive_forward`** — following outgoing edges via the owner field
///   (e.g. member→group direction for `Presenter`, keyed by `FIELD_GROUPS_id`).
/// - **`inclusive_reverse`** — following incoming edges via the target field
///   (e.g. group→member direction for `Presenter`, keyed by `FIELD_MEMBERS_id`).
///
/// Each entry is a `Box<[NonNilUuid]>` (immutable after insertion) keyed by
/// `start_uuid`. The field ID identifies which direction is being cached.
#[derive(Debug, Default)]
pub struct HomoEdgeCache {
    /// source_uuid → all UUIDs transitively reachable via the forward field.
    inclusive_forward: HashMap<NonNilUuid, Box<[NonNilUuid]>>,
    /// target_uuid → all UUIDs transitively reachable via the reverse field.
    inclusive_reverse: HashMap<NonNilUuid, Box<[NonNilUuid]>>,
}

impl HomoEdgeCache {
    /// Return all UUIDs transitively reachable from `start` by following
    /// outgoing edges under `forward_field` (e.g. member→group direction).
    ///
    /// The `start` node itself is **not** included in the result.
    /// Computed and cached on first call; subsequent calls clone the cached slice.
    pub fn get_or_compute_forward(
        &mut self,
        edge_map: &RawEdgeMap,
        start: NonNilUuid,
        forward_field: FieldId,
    ) -> Vec<NonNilUuid> {
        self.inclusive_forward
            .entry(start)
            .or_insert_with(|| {
                transitive_neighbors(edge_map, start, forward_field).into_boxed_slice()
            })
            .to_vec()
    }

    /// Return all UUIDs transitively reachable from `start` by following
    /// incoming edges under `reverse_field` (e.g. group→member direction).
    ///
    /// The `start` node itself is **not** included in the result.
    /// Computed and cached on first call; subsequent calls clone the cached slice.
    pub fn get_or_compute_reverse(
        &mut self,
        edge_map: &RawEdgeMap,
        start: NonNilUuid,
        reverse_field: FieldId,
    ) -> Vec<NonNilUuid> {
        self.inclusive_reverse
            .entry(start)
            .or_insert_with(|| {
                transitive_neighbors(edge_map, start, reverse_field).into_boxed_slice()
            })
            .to_vec()
    }
}

// ── Internal traversal ────────────────────────────────────────────────────────

/// BFS transitive closure starting from `start`, following all edges stored
/// under `field_id` at each visited node.  Returns all reachable entity UUIDs
/// excluding `start` itself.  Handles cycles via a visited set.
fn transitive_neighbors(
    edge_map: &RawEdgeMap,
    start: NonNilUuid,
    field_id: FieldId,
) -> Vec<NonNilUuid> {
    let mut visited: HashSet<NonNilUuid> = HashSet::new();
    visited.insert(start); // prevent re-queuing start or looping back through it
    let mut queue = vec![start];
    let mut result = Vec::new();

    while let Some(curr) = queue.pop() {
        for node in edge_map.neighbors_for_field(curr, field_id) {
            let uuid = node.entity;
            if visited.insert(uuid) {
                result.push(uuid);
                queue.push(uuid);
            }
        }
    }

    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;
    use crate::field::{FieldDescriptor, FieldDescriptorAny};
    use crate::field_node_id::FieldNodeId;
    use crate::field_set::FieldSet;
    use crate::value::{
        CrdtFieldType, FieldCardinality, FieldType, FieldTypeItem, ValidationError,
    };
    use uuid::{NonNilUuid, Uuid};

    struct TypeA;
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

    // Two static fields used as forward / reverse directions.
    static FIELD_FWD: FieldDescriptor<TypeA> = FieldDescriptor {
        name: "groups",
        display: "Groups",
        description: "Forward field",
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

    static FIELD_REV: FieldDescriptor<TypeA> = FieldDescriptor {
        name: "members",
        display: "Members",
        description: "Reverse field",
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

    fn fwd() -> FieldId {
        FieldId::of::<TypeA>(&FIELD_FWD)
    }
    fn rev() -> FieldId {
        FieldId::of::<TypeA>(&FIELD_REV)
    }

    fn nnu(n: u128) -> NonNilUuid {
        NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
    }

    /// Build a bidirectional edge: member's `fwd()` ↔ group's `rev()`.
    fn add_member_group(map: &mut RawEdgeMap, member: u128, group: u128) {
        map.add_edge(
            FieldNodeId::new(fwd(), nnu(member)),
            FieldNodeId::new(rev(), nnu(group)),
        );
    }

    #[test]
    fn test_cache_invalidation_via_option() {
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        add_member_group(&mut edge_map, 1, 2);
        let result = cache.get_or_compute_forward(&edge_map, nnu(1), fwd());
        assert_eq!(result, vec![nnu(2)]);

        // Simulate invalidation: drop cache and rebuild
        cache = HomoEdgeCache::default();
        add_member_group(&mut edge_map, 2, 3);
        let result = cache.get_or_compute_forward(&edge_map, nnu(1), fwd());
        assert!(result.contains(&nnu(2)));
        assert!(result.contains(&nnu(3)));
    }

    #[test]
    fn test_transitive_closure_forward() {
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // Chain: 1 → 2 → 3 (forward via fwd() field)
        add_member_group(&mut edge_map, 1, 2);
        add_member_group(&mut edge_map, 2, 3);

        let result = cache.get_or_compute_forward(&edge_map, nnu(1), fwd());
        assert!(result.contains(&nnu(2)), "should reach direct neighbor 2");
        assert!(
            result.contains(&nnu(3)),
            "should reach transitive neighbor 3"
        );
        assert!(!result.contains(&nnu(1)), "start node not included");
    }

    #[test]
    fn test_transitive_closure_reverse() {
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // Chain: 3 → 2 → 1 (forward); reverse from 1 via rev() reaches 2 and 3
        add_member_group(&mut edge_map, 3, 2);
        add_member_group(&mut edge_map, 2, 1);

        let result = cache.get_or_compute_reverse(&edge_map, nnu(1), rev());
        assert!(result.contains(&nnu(2)));
        assert!(result.contains(&nnu(3)));
        assert!(!result.contains(&nnu(1)), "start node not included");
    }

    #[test]
    fn test_cycle_handling() {
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // Cycle: 1 → 2, 2 → 1 (via fwd field)
        add_member_group(&mut edge_map, 1, 2);
        add_member_group(&mut edge_map, 2, 1);

        let result = cache.get_or_compute_forward(&edge_map, nnu(1), fwd());
        assert!(result.contains(&nnu(2)));
        assert!(
            !result.contains(&nnu(1)),
            "start node not included even in cycle"
        );
    }

    #[test]
    fn test_empty_result() {
        let mut cache = HomoEdgeCache::default();
        let edge_map = RawEdgeMap::default();

        let result = cache.get_or_compute_forward(&edge_map, nnu(1), fwd());
        assert!(result.is_empty());
    }

    #[test]
    fn test_cached_result_reused() {
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();
        add_member_group(&mut edge_map, 1, 2);

        let r1 = cache.get_or_compute_forward(&edge_map, nnu(1), fwd());
        // Mutate edge_map WITHOUT invalidating cache — cached result should stay
        add_member_group(&mut edge_map, 2, 3);
        let r2 = cache.get_or_compute_forward(&edge_map, nnu(1), fwd());
        assert_eq!(r1, r2, "stale cache should be returned unchanged");
    }

    #[test]
    fn test_presenter_group_scenario() {
        // Alice (1) and Bob (2) are members of Team A (3).
        // Team A (3) is a member of Division C (4).
        // Division C (4) is a member of Corp D (5).
        // Alice (1) is also a member of Club E (6).
        // Team B (7) is a member of Division C (4) — should NOT appear in Team A's members.
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // member → group direction (forward via fwd field, reverse via rev field)
        add_member_group(&mut edge_map, 1, 3); // Alice → Team A
        add_member_group(&mut edge_map, 2, 3); // Bob → Team A
        add_member_group(&mut edge_map, 3, 4); // Team A → Division C
        add_member_group(&mut edge_map, 4, 5); // Division C → Corp D
        add_member_group(&mut edge_map, 1, 6); // Alice → Club E
        add_member_group(&mut edge_map, 7, 4); // Team B → Division C

        // Inclusive groups of Team A (forward from 3 via fwd field): Division C, Corp D
        let groups_of_team_a = cache.get_or_compute_forward(&edge_map, nnu(3), fwd());
        assert!(groups_of_team_a.contains(&nnu(4)), "Division C");
        assert!(groups_of_team_a.contains(&nnu(5)), "Corp D");
        assert!(!groups_of_team_a.contains(&nnu(1)), "not Alice");
        assert!(!groups_of_team_a.contains(&nnu(7)), "not Team B");

        // Inclusive members of Team A (reverse from 3 via rev field): Alice, Bob only
        let members_of_team_a = cache.get_or_compute_reverse(&edge_map, nnu(3), rev());
        assert!(members_of_team_a.contains(&nnu(1)), "Alice");
        assert!(members_of_team_a.contains(&nnu(2)), "Bob");
        assert!(
            !members_of_team_a.contains(&nnu(7)),
            "Team B not a member of Team A"
        );
        assert!(!members_of_team_a.contains(&nnu(6)), "Club E not a member");

        // Inclusive groups of Alice (forward from 1 via fwd field): Team A, Club E, Division C, Corp D
        let groups_of_alice = cache.get_or_compute_forward(&edge_map, nnu(1), fwd());
        assert!(groups_of_alice.contains(&nnu(3)), "Team A");
        assert!(groups_of_alice.contains(&nnu(6)), "Club E");
        assert!(groups_of_alice.contains(&nnu(4)), "Division C (via Team A)");
        assert!(groups_of_alice.contains(&nnu(5)), "Corp D (via Division C)");
        assert!(
            !groups_of_alice.contains(&nnu(7)),
            "Team B not a group of Alice"
        );
    }
}
