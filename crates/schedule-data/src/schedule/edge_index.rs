/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge index for fast endpoint-based lookups.
//!
//! Each edge type gets one [`EdgeIndex`] that tracks outgoing (from→edge) and
//! incoming (to→edge) relationships.  The index stores **edge entity UUIDs**,
//! so callers can retrieve full edge data from [`super::EntityStorage`].

use std::collections::HashMap;

use uuid::NonNilUuid;

/// Bidirectional index for one edge type.
///
/// Maintains two maps: `outgoing` (from-endpoint → edge UUIDs) and
/// `incoming` (to-endpoint → edge UUIDs).  Must be kept in sync with
/// the corresponding entity `HashMap` in [`super::EntityStorage`].
///
/// # Invariants
///
/// - Every edge UUID in `outgoing` must also appear in `incoming` and
///   vice versa.
/// - The maps are empty-cleaned: when the last edge for a key is removed,
///   the key itself is removed from the map.
#[derive(Debug, Clone, Default)]
pub struct EdgeIndex {
    /// from-endpoint UUID → edge entity UUIDs leaving that node
    outgoing: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// to-endpoint UUID → edge entity UUIDs arriving at that node
    incoming: HashMap<NonNilUuid, Vec<NonNilUuid>>,
}

impl EdgeIndex {
    /// Create a new empty edge index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an edge in both outgoing and incoming indexes.
    pub fn add(&mut self, from: NonNilUuid, to: NonNilUuid, edge_uuid: NonNilUuid) {
        self.outgoing.entry(from).or_default().push(edge_uuid);
        self.incoming.entry(to).or_default().push(edge_uuid);
    }

    /// Remove an edge from both outgoing and incoming indexes.
    pub fn remove(&mut self, from: NonNilUuid, to: NonNilUuid, edge_uuid: NonNilUuid) {
        if let Some(edges) = self.outgoing.get_mut(&from) {
            edges.retain(|&e| e != edge_uuid);
            if edges.is_empty() {
                self.outgoing.remove(&from);
            }
        }
        if let Some(edges) = self.incoming.get_mut(&to) {
            edges.retain(|&e| e != edge_uuid);
            if edges.is_empty() {
                self.incoming.remove(&to);
            }
        }
    }

    /// Edge entity UUIDs leaving `from`.
    pub fn outgoing(&self, from: NonNilUuid) -> &[NonNilUuid] {
        self.outgoing.get(&from).map_or(&[], Vec::as_slice)
    }

    /// Edge entity UUIDs arriving at `to`.
    pub fn incoming(&self, to: NonNilUuid) -> &[NonNilUuid] {
        self.incoming.get(&to).map_or(&[], Vec::as_slice)
    }

    /// Total number of indexed edges (counted by outgoing entries).
    pub fn len(&self) -> usize {
        self.outgoing.values().map(Vec::len).sum()
    }

    /// Whether this index contains no edges.
    pub fn is_empty(&self) -> bool {
        self.outgoing.is_empty()
    }

    /// Number of distinct from-endpoints.
    pub fn from_count(&self) -> usize {
        self.outgoing.len()
    }

    /// Number of distinct to-endpoints.
    pub fn to_count(&self) -> usize {
        self.incoming.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn nn(b: u8) -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b,
            ]))
        }
    }

    #[test]
    fn edge_index_new_is_empty() {
        let idx = EdgeIndex::new();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn edge_index_add_and_lookup() {
        let mut idx = EdgeIndex::new();
        idx.add(nn(1), nn(2), nn(10));
        assert_eq!(idx.outgoing(nn(1)), &[nn(10)]);
        assert_eq!(idx.incoming(nn(2)), &[nn(10)]);
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn edge_index_multiple_outgoing() {
        let mut idx = EdgeIndex::new();
        idx.add(nn(1), nn(2), nn(10));
        idx.add(nn(1), nn(3), nn(11));
        assert_eq!(idx.outgoing(nn(1)), &[nn(10), nn(11)]);
        assert_eq!(idx.incoming(nn(2)), &[nn(10)]);
        assert_eq!(idx.incoming(nn(3)), &[nn(11)]);
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn edge_index_multiple_incoming() {
        let mut idx = EdgeIndex::new();
        idx.add(nn(1), nn(3), nn(10));
        idx.add(nn(2), nn(3), nn(11));
        assert_eq!(idx.incoming(nn(3)), &[nn(10), nn(11)]);
        assert_eq!(idx.outgoing(nn(1)), &[nn(10)]);
        assert_eq!(idx.outgoing(nn(2)), &[nn(11)]);
    }

    #[test]
    fn edge_index_remove() {
        let mut idx = EdgeIndex::new();
        idx.add(nn(1), nn(2), nn(10));
        idx.add(nn(1), nn(3), nn(11));
        idx.remove(nn(1), nn(2), nn(10));
        assert_eq!(idx.outgoing(nn(1)), &[nn(11)]);
        assert!(idx.incoming(nn(2)).is_empty());
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn edge_index_remove_last_cleans_key() {
        let mut idx = EdgeIndex::new();
        idx.add(nn(1), nn(2), nn(10));
        idx.remove(nn(1), nn(2), nn(10));
        assert!(idx.is_empty());
        assert_eq!(idx.from_count(), 0);
        assert_eq!(idx.to_count(), 0);
    }

    #[test]
    fn edge_index_remove_nonexistent_is_harmless() {
        let mut idx = EdgeIndex::new();
        idx.remove(nn(1), nn(2), nn(99));
        assert!(idx.is_empty());
    }

    #[test]
    fn edge_index_self_loop() {
        let mut idx = EdgeIndex::new();
        idx.add(nn(1), nn(1), nn(10));
        assert_eq!(idx.outgoing(nn(1)), &[nn(10)]);
        assert_eq!(idx.incoming(nn(1)), &[nn(10)]);
        assert_eq!(idx.len(), 1);
        idx.remove(nn(1), nn(1), nn(10));
        assert!(idx.is_empty());
    }

    #[test]
    fn edge_index_from_and_to_counts() {
        let mut idx = EdgeIndex::new();
        idx.add(nn(1), nn(3), nn(10));
        idx.add(nn(2), nn(3), nn(11));
        assert_eq!(idx.from_count(), 2);
        assert_eq!(idx.to_count(), 1);
    }
}
