/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`TransitiveEdgeCache`] — transitive-edge relationship cache.
//!
//! Caches transitive closures of edges between the same entity type (e.g. presenter
//! group membership) to enable efficient queries like "all groups a presenter
//! belongs to" or "all members of a group including nested groups".
//!
//! The cache is stored as `Option<TransitiveEdgeCache>` on [`crate::schedule::Schedule`]
//! via a `RefCell`. Setting it to `None` invalidates the cache; it is rebuilt
//! lazily per-entry on the next query.
//!
//! ## What is NOT cached here
//!
//! Heterogeneous-edge transitive queries (e.g. Panel → Inclusive Presenters)
//! require multi-type traversal and are implemented as field read functions in
//! the entity modules ([`crate::tables::panel`], [`crate::tables::presenter`]), composed from
//! `inclusive_edges_from` / `inclusive_edges_to` calls on the schedule.

use crate::edge::id::FullEdge;
use crate::edge::map::RawEdgeMap;
use crate::entity::{DynamicEntityId, EntityUuid};
use std::collections::{HashMap, HashSet};
use uuid::NonNilUuid;

// ── TransitiveEdgeCache ───────────────────────────────────────────────────────────

/// Cached transitive-edge relationships.
///
/// Only the computed transitive maps are stored here; direct edges are always
/// queried live from [`RawEdgeMap`]. Entries are populated lazily and remain
/// valid until the cache is set to `None` (invalidated) by any transitive-edge
/// mutation.
///
/// Entries are keyed by `(FullEdge, NonNilUuid)` — the edge encodes traversal
/// direction (forward and reverse use different `FullEdge` orientations),
/// while the UUID is the starting node. Multiple independent transitive-edge
/// relationships can share one cache without key collision.
#[derive(Debug, Default)]
pub struct TransitiveEdgeCache {
    /// `(edge, start_uuid)` → all UUIDs transitively reachable by following
    /// the edge from the starting node.
    cache: HashMap<(FullEdge, NonNilUuid), Box<[NonNilUuid]>>,
}

impl TransitiveEdgeCache {
    /// Return all UUIDs transitively reachable from `start` by following `edge`.
    ///
    /// The `start` node itself is **not** included in the result. Computed and
    /// cached on first call; subsequent calls clone the cached slice.
    pub fn get_or_compute(
        &mut self,
        edge_map: &RawEdgeMap,
        start: impl DynamicEntityId,
        edge: FullEdge,
    ) -> Vec<NonNilUuid> {
        let start_uuid = start.entity_uuid();
        self.cache
            .entry((edge, start_uuid))
            .or_insert_with(|| transitive_neighbors(edge_map, start_uuid, edge).into_boxed_slice())
            .to_vec()
    }
}

// ── Internal traversal ────────────────────────────────────────────────────────

/// BFS transitive closure starting from `start`, following `edge` at each visited node.
/// Returns all reachable entity UUIDs excluding `start` itself. Handles cycles via a
/// visited set.
fn transitive_neighbors(
    edge_map: &RawEdgeMap,
    start: NonNilUuid,
    edge: FullEdge,
) -> Vec<NonNilUuid> {
    let mut visited: HashSet<NonNilUuid> = HashSet::new();
    visited.insert(start); // prevent re-queuing start or looping back through it
    let mut queue = vec![start];
    let mut result = Vec::new();

    // Get entity type name from the edge (homogeneous edges have same type on both sides)
    let type_name = edge.near.as_named_field().entity_type_name();

    while let Some(curr) = queue.pop() {
        // SAFETY: The transitive closure is only used for homogeneous edges,
        // so all nodes in the traversal have the same entity type.
        let curr_id = unsafe { crate::entity::RuntimeEntityId::new_unchecked(curr, type_name) };
        for node in edge_map.neighbors(curr_id, edge) {
            let uuid = node.entity_uuid();
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
    use crate::crdt::CrdtFieldType;
    use crate::edge::id::FullEdge;
    use crate::edge::EdgeKind;
    use crate::entity::{EntityId, EntityType};
    use crate::field::set::FieldSet;
    use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor};
    use crate::value::{FieldCardinality, FieldType, FieldTypeItem, ValidationError};
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
        data: CommonFieldData {
            name: "owner",
            display: "Owner",
            description: "Forward field",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
            example: "",
            order: 0,
        },
        required: false,
        edge_kind: EdgeKind::Owner {
            target_field: &FIELD_REV,
            exclusive_with: None,
        },
        crdt_type: CrdtFieldType::Derived,
        cb: FieldCallbacks {
            read_fn: None,
            write_fn: None,
            verify_fn: None,
        },
    };

    static FIELD_REV: FieldDescriptor<TypeA> = FieldDescriptor {
        data: CommonFieldData {
            name: "members",
            display: "Members",
            description: "Reverse field",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
            example: "",
            order: 1,
        },
        required: false,
        edge_kind: EdgeKind::Target { source_fields: &[] },
        crdt_type: CrdtFieldType::Derived,
        cb: FieldCallbacks {
            read_fn: None,
            write_fn: None,
            verify_fn: None,
        },
    };

    fn fwd() -> FullEdge {
        FIELD_FWD.edge_to(&FIELD_REV)
    }
    fn rev() -> FullEdge {
        FIELD_REV.edge_to(&FIELD_FWD)
    }

    fn nnu(n: u128) -> NonNilUuid {
        NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
    }

    fn id(n: u128) -> EntityId<TypeA> {
        // SAFETY: Test fixtures use valid UUIDs for TypeA.
        unsafe { EntityId::new_unchecked(nnu(n)) }
    }

    /// Build a bidirectional edge: member's `fwd()` ↔ group's `rev()`.
    fn add_member_group(map: &mut RawEdgeMap, member: u128, group: u128) {
        // SAFETY: Test fixtures use matching entity types for their fields.
        let fwd_edge = fwd();
        let member_id =
            unsafe { crate::entity::RuntimeEntityId::new_unchecked(nnu(member), "type_a") };
        let group_id =
            unsafe { crate::entity::RuntimeEntityId::new_unchecked(nnu(group), "type_a") };
        map.add_edge(member_id, fwd_edge, std::iter::once(group_id))
            .expect("edge type validation failed");
    }

    #[test]
    fn test_cache_invalidation_via_option() {
        let mut cache = TransitiveEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        add_member_group(&mut edge_map, 1, 2);
        let result = cache.get_or_compute(&edge_map, id(1), fwd());
        assert_eq!(result, vec![nnu(2)]);

        // Simulate invalidation: drop cache and rebuild
        cache = TransitiveEdgeCache::default();
        add_member_group(&mut edge_map, 2, 3);
        let result = cache.get_or_compute(&edge_map, id(1), fwd());
        assert!(result.contains(&nnu(2)));
        assert!(result.contains(&nnu(3)));
    }

    #[test]
    fn test_transitive_closure_forward() {
        let mut cache = TransitiveEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // Chain: 1 → 2 → 3 (forward via fwd() field)
        add_member_group(&mut edge_map, 1, 2);
        add_member_group(&mut edge_map, 2, 3);

        let result = cache.get_or_compute(&edge_map, id(1), fwd());
        assert!(result.contains(&nnu(2)), "should reach direct neighbor 2");
        assert!(
            result.contains(&nnu(3)),
            "should reach transitive neighbor 3"
        );
        assert!(!result.contains(&nnu(1)), "start node not included");
    }

    #[test]
    fn test_transitive_closure_reverse() {
        let mut cache = TransitiveEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // Chain: 3 → 2 → 1 (forward); reverse from 1 via rev() reaches 2 and 3
        add_member_group(&mut edge_map, 3, 2);
        add_member_group(&mut edge_map, 2, 1);

        let result = cache.get_or_compute(&edge_map, id(1), rev());
        assert!(result.contains(&nnu(2)));
        assert!(result.contains(&nnu(3)));
        assert!(!result.contains(&nnu(1)), "start node not included");
    }

    #[test]
    fn test_cycle_handling() {
        let mut cache = TransitiveEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // Cycle: 1 → 2, 2 → 1 (via fwd field)
        add_member_group(&mut edge_map, 1, 2);
        add_member_group(&mut edge_map, 2, 1);

        let result = cache.get_or_compute(&edge_map, id(1), fwd());
        assert!(result.contains(&nnu(2)));
        assert!(
            !result.contains(&nnu(1)),
            "start node not included even in cycle"
        );
    }

    #[test]
    fn test_empty_result() {
        let mut cache = TransitiveEdgeCache::default();
        let edge_map = RawEdgeMap::default();

        let result = cache.get_or_compute(&edge_map, id(1), fwd());
        assert!(result.is_empty());
    }

    #[test]
    fn test_cached_result_reused() {
        let mut cache = TransitiveEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();
        add_member_group(&mut edge_map, 1, 2);

        let r1 = cache.get_or_compute(&edge_map, id(1), fwd());
        // Mutate edge_map WITHOUT invalidating cache — cached result should stay
        add_member_group(&mut edge_map, 2, 3);
        let r2 = cache.get_or_compute(&edge_map, id(1), fwd());
        assert_eq!(r1, r2, "stale cache should be returned unchanged");
    }

    #[test]
    fn test_presenter_group_scenario() {
        // Alice (1) and Bob (2) are members of Team A (3).
        // Team A (3) is a member of Division C (4).
        // Division C (4) is a member of Corp D (5).
        // Alice (1) is also a member of Club E (6).
        // Team B (7) is a member of Division C (4) — should NOT appear in Team A's members.
        let mut cache = TransitiveEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // member → group direction (forward via fwd field, reverse via rev field)
        add_member_group(&mut edge_map, 1, 3); // Alice → Team A
        add_member_group(&mut edge_map, 2, 3); // Bob → Team A
        add_member_group(&mut edge_map, 3, 4); // Team A → Division C
        add_member_group(&mut edge_map, 4, 5); // Division C → Corp D
        add_member_group(&mut edge_map, 1, 6); // Alice → Club E
        add_member_group(&mut edge_map, 7, 4); // Team B → Division C

        // Inclusive groups of Team A (forward from 3 via fwd field): Division C, Corp D
        let groups_of_team_a = cache.get_or_compute(&edge_map, id(3), fwd());
        assert!(groups_of_team_a.contains(&nnu(4)), "Division C");
        assert!(groups_of_team_a.contains(&nnu(5)), "Corp D");
        assert!(!groups_of_team_a.contains(&nnu(1)), "not Alice");
        assert!(!groups_of_team_a.contains(&nnu(7)), "not Team B");

        // Inclusive members of Team A (reverse from 3 via rev field): Alice, Bob only
        let members_of_team_a = cache.get_or_compute(&edge_map, id(3), rev());
        assert!(members_of_team_a.contains(&nnu(1)), "Alice");
        assert!(members_of_team_a.contains(&nnu(2)), "Bob");
        assert!(
            !members_of_team_a.contains(&nnu(7)),
            "Team B not a member of Team A"
        );
        assert!(!members_of_team_a.contains(&nnu(6)), "Club E not a member");

        // Inclusive groups of Alice (forward from 1 via fwd field): Team A, Club E, Division C, Corp D
        let groups_of_alice = cache.get_or_compute(&edge_map, id(1), fwd());
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
