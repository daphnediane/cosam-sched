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
use std::collections::{HashMap, HashSet};
use uuid::NonNilUuid;

// ── HomoEdgeCache ─────────────────────────────────────────────────────────────────

/// Cached transitive homogeneous-edge relationships.
///
/// Only the computed transitive maps are stored here; direct edges are always
/// queried live from [`RawEdgeMap`]. Entries are populated lazily and remain
/// valid until the cache is set to `None` (invalidated) by any homogeneous
/// edge mutation.
///
/// Two traversal directions are cached independently:
///
/// - **`inclusive_forward`** — following outgoing homogeneous edges
///   (e.g. member→group direction for `Presenter`).
/// - **`inclusive_reverse`** — following incoming homogeneous edges
///   (e.g. group→member direction for `Presenter`).
///
/// Each entry is a `Box<[NonNilUuid]>` (immutable after insertion) keyed by
/// `start_uuid`. The type name is not part of the key because UUIDs are unique
/// across all entity types, so a given UUID belongs to exactly one type.
#[derive(Debug, Default)]
pub struct HomoEdgeCache {
    /// source_uuid → all UUIDs transitively reachable via forward homogeneous edges.
    inclusive_forward: HashMap<NonNilUuid, Box<[NonNilUuid]>>,
    /// target_uuid → all UUIDs transitively reachable via reverse homogeneous edges.
    inclusive_reverse: HashMap<NonNilUuid, Box<[NonNilUuid]>>,
}

impl HomoEdgeCache {
    /// Return all UUIDs transitively reachable from `start` by following
    /// outgoing homogeneous edges of `type_name` (e.g. member→group direction).
    ///
    /// The `start` node itself is **not** included in the result.
    /// Computed and cached on first call; subsequent calls clone the cached slice.
    pub fn get_or_compute_forward(
        &mut self,
        edge_map: &RawEdgeMap,
        start: NonNilUuid,
        type_name: &'static str,
    ) -> Vec<NonNilUuid> {
        self.inclusive_forward
            .entry(start)
            .or_insert_with(|| {
                transitive_neighbors(edge_map, start, type_name, NeighborDir::Forward)
                    .into_boxed_slice()
            })
            .to_vec()
    }

    /// Return all UUIDs transitively reachable from `start` by following
    /// incoming homogeneous edges of `type_name` (e.g. group→member direction).
    ///
    /// The `start` node itself is **not** included in the result.
    /// Computed and cached on first call; subsequent calls clone the cached slice.
    pub fn get_or_compute_reverse(
        &mut self,
        edge_map: &RawEdgeMap,
        start: NonNilUuid,
        type_name: &'static str,
    ) -> Vec<NonNilUuid> {
        self.inclusive_reverse
            .entry(start)
            .or_insert_with(|| {
                transitive_neighbors(edge_map, start, type_name, NeighborDir::Reverse)
                    .into_boxed_slice()
            })
            .to_vec()
    }
}

// ── Internal traversal ────────────────────────────────────────────────────────

enum NeighborDir {
    Forward,
    Reverse,
}

/// BFS transitive closure starting from `start`, following homogeneous edges
/// in the given direction, filtered to `type_name`. Returns all reachable
/// nodes excluding `start` itself. Handles cycles via a visited set.
fn transitive_neighbors(
    edge_map: &RawEdgeMap,
    start: NonNilUuid,
    type_name: &'static str,
    dir: NeighborDir,
) -> Vec<NonNilUuid> {
    let mut visited: HashSet<NonNilUuid> = HashSet::new();
    visited.insert(start); // prevent re-queuing start or looping back through it
    let mut queue = vec![start];
    let mut result = Vec::new();

    while let Some(curr) = queue.pop() {
        let neighbors = match dir {
            NeighborDir::Forward => edge_map.neighbors(curr),
            NeighborDir::Reverse => edge_map.homo_reverse(curr),
        };
        for rid in neighbors {
            if rid.type_name() == type_name {
                let uuid = rid.uuid();
                if visited.insert(uuid) {
                    result.push(uuid);
                    queue.push(uuid);
                }
            }
        }
    }

    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityType, RuntimeEntityId};
    use crate::field_set::FieldSet;
    use crate::value::ValidationError;
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

    fn nnu(n: u128) -> NonNilUuid {
        NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
    }

    fn rid_a(n: u128) -> RuntimeEntityId {
        unsafe { RuntimeEntityId::from_uuid(nnu(n), TypeA::TYPE_NAME) }
    }

    #[test]
    fn test_cache_invalidation_via_option() {
        // HomoEdgeCache is held as Option<HomoEdgeCache> on Schedule.
        // Setting to None invalidates; the cache is rebuilt on next access.
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        edge_map.add_homo(rid_a(1), rid_a(2));
        let result = cache.get_or_compute_forward(&edge_map, nnu(1), TypeA::TYPE_NAME);
        assert_eq!(result, vec![nnu(2)]);

        // Simulate invalidation: drop cache and rebuild
        cache = HomoEdgeCache::default();
        edge_map.add_homo(rid_a(2), rid_a(3));
        let result = cache.get_or_compute_forward(&edge_map, nnu(1), TypeA::TYPE_NAME);
        assert!(result.contains(&nnu(2)));
        assert!(result.contains(&nnu(3)));
    }

    #[test]
    fn test_transitive_closure_forward() {
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // Chain: 1 → 2 → 3
        edge_map.add_homo(rid_a(1), rid_a(2));
        edge_map.add_homo(rid_a(2), rid_a(3));

        let result = cache.get_or_compute_forward(&edge_map, nnu(1), TypeA::TYPE_NAME);
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

        // Chain: 3 → 2 → 1 (forward); reverse from 1 reaches 2 and 3
        edge_map.add_homo(rid_a(3), rid_a(2));
        edge_map.add_homo(rid_a(2), rid_a(1));

        let result = cache.get_or_compute_reverse(&edge_map, nnu(1), TypeA::TYPE_NAME);
        assert!(result.contains(&nnu(2)));
        assert!(result.contains(&nnu(3)));
        assert!(!result.contains(&nnu(1)), "start node not included");
    }

    #[test]
    fn test_cycle_handling() {
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();

        // Cycle: 1 → 2, 2 → 1
        edge_map.add_homo(rid_a(1), rid_a(2));
        edge_map.add_homo(rid_a(2), rid_a(1));

        // Should not infinite loop; 2 is reachable from 1
        let result = cache.get_or_compute_forward(&edge_map, nnu(1), TypeA::TYPE_NAME);
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

        let result = cache.get_or_compute_forward(&edge_map, nnu(1), TypeA::TYPE_NAME);
        assert!(result.is_empty());
    }

    #[test]
    fn test_cached_result_reused() {
        let mut cache = HomoEdgeCache::default();
        let mut edge_map = RawEdgeMap::default();
        edge_map.add_homo(rid_a(1), rid_a(2));

        let r1 = cache.get_or_compute_forward(&edge_map, nnu(1), TypeA::TYPE_NAME);
        // Mutate edge_map WITHOUT invalidating cache — cached result should stay
        edge_map.add_homo(rid_a(2), rid_a(3));
        let r2 = cache.get_or_compute_forward(&edge_map, nnu(1), TypeA::TYPE_NAME);
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

        // member → group direction (forward homogeneous edges)
        edge_map.add_homo(rid_a(1), rid_a(3)); // Alice → Team A
        edge_map.add_homo(rid_a(2), rid_a(3)); // Bob → Team A
        edge_map.add_homo(rid_a(3), rid_a(4)); // Team A → Division C
        edge_map.add_homo(rid_a(4), rid_a(5)); // Division C → Corp D
        edge_map.add_homo(rid_a(1), rid_a(6)); // Alice → Club E
        edge_map.add_homo(rid_a(7), rid_a(4)); // Team B → Division C

        // Inclusive groups of Team A (forward from 3): Division C, Corp D
        let groups_of_team_a = cache.get_or_compute_forward(&edge_map, nnu(3), TypeA::TYPE_NAME);
        assert!(groups_of_team_a.contains(&nnu(4)), "Division C");
        assert!(groups_of_team_a.contains(&nnu(5)), "Corp D");
        assert!(!groups_of_team_a.contains(&nnu(1)), "not Alice");
        assert!(!groups_of_team_a.contains(&nnu(7)), "not Team B");

        // Inclusive members of Team A (reverse from 3): Alice, Bob only
        // (NOT Team B — Team B is sibling in Division C, not a member of Team A)
        let members_of_team_a = cache.get_or_compute_reverse(&edge_map, nnu(3), TypeA::TYPE_NAME);
        assert!(members_of_team_a.contains(&nnu(1)), "Alice");
        assert!(members_of_team_a.contains(&nnu(2)), "Bob");
        assert!(
            !members_of_team_a.contains(&nnu(7)),
            "Team B not a member of Team A"
        );
        assert!(!members_of_team_a.contains(&nnu(6)), "Club E not a member");

        // Inclusive groups of Alice (forward from 1): Team A, Club E, Division C, Corp D
        let groups_of_alice = cache.get_or_compute_forward(&edge_map, nnu(1), TypeA::TYPE_NAME);
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
