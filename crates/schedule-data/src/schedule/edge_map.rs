/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Bidirectional edge index for many-to-many relationships.
//!
//! [`EdgeMap<L, R>`] maintains two `HashMap` indexes simultaneously —
//! `by_left` and `by_right` — so that either endpoint can be used as a
//! lookup key in O(1) time. All mutating operations keep both indexes in sync.
//!
//! Use [`EdgeMap`] anywhere the relationship needs reverse lookups from both
//! sides, including self-referential relationships where `L = R`
//! (e.g., presenter group-member relationships).

use std::collections::HashMap;

use crate::entity::TypedId;

/// Bidirectional edge index for many-to-many relationships.
///
/// Maintains two `HashMap` indexes simultaneously:
///
/// - `by_left`: maps each left ID to all of its associated right IDs
/// - `by_right`: maps each right ID to all of its associated left IDs
///
/// All mutating operations (`add`, `remove`, `update_by_left`,
/// `update_by_right`, `clear_by_left`, `clear_by_right`) keep both indexes
/// in sync automatically.
///
/// The type is deliberately parameterized so that `L = R` is allowed, which
/// supports self-referential relationships (e.g.,
/// `EdgeMap<PresenterId, PresenterId>` for group-member edges where both
/// endpoints share the same type).
///
/// # Type Parameters
///
/// - `L`: the left-endpoint ID type (e.g., `PanelTypeId`, group `PresenterId`)
/// - `R`: the right-endpoint ID type (e.g., `PanelId`, member `PresenterId`)
#[derive(Debug, Clone)]
pub struct EdgeMap<L, R> {
    by_left: HashMap<L, Vec<R>>,
    by_right: HashMap<R, Vec<L>>,
}

impl<L, R> Default for EdgeMap<L, R> {
    fn default() -> Self {
        Self {
            by_left: HashMap::new(),
            by_right: HashMap::new(),
        }
    }
}

impl<L, R> EdgeMap<L, R>
where
    L: TypedId + std::hash::Hash + Eq + Copy,
    R: TypedId + std::hash::Hash + Eq + Copy,
{
    /// Creates an empty `EdgeMap`.
    pub fn new() -> Self {
        Self {
            by_left: HashMap::new(),
            by_right: HashMap::new(),
        }
    }

    /// Creates an empty `EdgeMap` pre-allocated for approximately `capacity`
    /// distinct left-endpoint entries.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            by_left: HashMap::with_capacity(capacity),
            by_right: HashMap::with_capacity(capacity),
        }
    }

    // -----------------------------------------------------------------------
    // Pair operations
    // -----------------------------------------------------------------------

    /// Insert an edge `(left, right)` into both indexes.
    ///
    /// Duplicate edges are allowed; use [`contains`](Self::contains) before
    /// calling if de-duplication is needed.
    pub fn add(&mut self, left: L, right: R) {
        self.by_left.entry(left).or_default().push(right);
        self.by_right.entry(right).or_default().push(left);
    }

    /// Remove all edges matching `(left, right)` from both indexes.
    ///
    /// Silently does nothing if the edge does not exist.
    pub fn remove(&mut self, left: &L, right: &R) {
        if let Some(rights) = self.by_left.get_mut(left) {
            rights.retain(|r| r != right);
            if rights.is_empty() {
                self.by_left.remove(left);
            }
        }
        if let Some(lefts) = self.by_right.get_mut(right) {
            lefts.retain(|l| l != left);
            if lefts.is_empty() {
                self.by_right.remove(right);
            }
        }
    }

    /// Returns `true` if the edge `(left, right)` exists.
    #[must_use]
    pub fn contains(&self, left: &L, right: &R) -> bool {
        self.by_left
            .get(left)
            .is_some_and(|rights| rights.contains(right))
    }

    // -----------------------------------------------------------------------
    // Left-keyed operations
    // -----------------------------------------------------------------------

    /// Returns a slice of right IDs associated with `left`.
    ///
    /// Returns an empty slice if `left` has no edges.
    #[must_use]
    pub fn by_left(&self, left: &L) -> &[R] {
        self.by_left.get(left).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Replace all right IDs for `left` with `new_rights`, keeping both
    /// indexes in sync.
    ///
    /// Returns a `Vec` of right IDs that were removed (present in the old
    /// set but not in `new_rights`).
    pub fn update_by_left(&mut self, left: L, new_rights: &[R]) -> Vec<R> {
        let old_rights: Vec<R> = self.by_left.get(&left).cloned().unwrap_or_default();
        let mut removed = Vec::new();

        for old_r in &old_rights {
            if !new_rights.contains(old_r) {
                if let Some(lefts) = self.by_right.get_mut(old_r) {
                    lefts.retain(|l| l != &left);
                    if lefts.is_empty() {
                        self.by_right.remove(old_r);
                    }
                }
                removed.push(*old_r);
            }
        }

        for new_r in new_rights {
            if !old_rights.contains(new_r) {
                self.by_right.entry(*new_r).or_default().push(left);
            }
        }

        if new_rights.is_empty() {
            self.by_left.remove(&left);
        } else {
            *self.by_left.entry(left).or_default() = new_rights.to_vec();
        }

        removed
    }

    /// Remove all edges where the left endpoint is `left`.
    ///
    /// Also removes `left` from all corresponding `by_right` entries.
    pub fn clear_by_left(&mut self, left: &L) {
        if let Some(rights) = self.by_left.remove(left) {
            for right in rights {
                if let Some(lefts) = self.by_right.get_mut(&right) {
                    lefts.retain(|l| l != left);
                    if lefts.is_empty() {
                        self.by_right.remove(&right);
                    }
                }
            }
        }
    }

    /// Returns `true` if `left` has at least one edge.
    #[must_use]
    pub fn contains_left(&self, left: &L) -> bool {
        self.by_left.contains_key(left)
    }

    // -----------------------------------------------------------------------
    // Right-keyed operations
    // -----------------------------------------------------------------------

    /// Returns a slice of left IDs associated with `right`.
    ///
    /// Returns an empty slice if `right` has no edges.
    #[must_use]
    pub fn by_right(&self, right: &R) -> &[L] {
        self.by_right.get(right).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Replace all left IDs for `right` with `new_lefts`, keeping both
    /// indexes in sync.
    ///
    /// Returns a `Vec` of left IDs that were removed.
    pub fn update_by_right(&mut self, right: R, new_lefts: &[L]) -> Vec<L> {
        let old_lefts: Vec<L> = self.by_right.get(&right).cloned().unwrap_or_default();
        let mut removed = Vec::new();

        for old_l in &old_lefts {
            if !new_lefts.contains(old_l) {
                if let Some(rights) = self.by_left.get_mut(old_l) {
                    rights.retain(|r| r != &right);
                    if rights.is_empty() {
                        self.by_left.remove(old_l);
                    }
                }
                removed.push(*old_l);
            }
        }

        for new_l in new_lefts {
            if !old_lefts.contains(new_l) {
                self.by_left.entry(*new_l).or_default().push(right);
            }
        }

        if new_lefts.is_empty() {
            self.by_right.remove(&right);
        } else {
            *self.by_right.entry(right).or_default() = new_lefts.to_vec();
        }

        removed
    }

    /// Remove all edges where the right endpoint is `right`.
    ///
    /// Also removes `right` from all corresponding `by_left` entries.
    pub fn clear_by_right(&mut self, right: &R) {
        if let Some(lefts) = self.by_right.remove(right) {
            for left in lefts {
                if let Some(rights) = self.by_left.get_mut(&left) {
                    rights.retain(|r| r != right);
                    if rights.is_empty() {
                        self.by_left.remove(&left);
                    }
                }
            }
        }
    }

    /// Returns `true` if `right` has at least one edge.
    #[must_use]
    pub fn contains_right(&self, right: &R) -> bool {
        self.by_right.contains_key(right)
    }

    // -----------------------------------------------------------------------
    // Iterators and miscellaneous
    // -----------------------------------------------------------------------

    /// Returns an iterator over all `(left, right)` edge pairs.
    ///
    /// Order is unspecified (HashMap iteration order).
    pub fn iter_edges(&self) -> impl Iterator<Item = (L, R)> + '_ {
        self.by_left
            .iter()
            .flat_map(|(l, rights)| rights.iter().map(move |r| (*l, *r)))
    }

    /// Returns an iterator over all left IDs that have at least one edge.
    pub fn left_keys(&self) -> impl Iterator<Item = L> + '_ {
        self.by_left.keys().copied()
    }

    /// Returns an iterator over all right IDs that have at least one edge.
    pub fn right_keys(&self) -> impl Iterator<Item = R> + '_ {
        self.by_right.keys().copied()
    }

    /// Returns the number of distinct left IDs that have at least one edge.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_left.len()
    }

    /// Returns `true` if no edges are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_left.is_empty()
    }

    /// Reserves capacity for at least `additional` more left-ID entries.
    pub fn reserve(&mut self, additional: usize) {
        self.by_left.reserve(additional);
        self.by_right.reserve(additional);
    }

    /// Shrinks both internal maps as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.by_left.shrink_to_fit();
        self.by_right.shrink_to_fit();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /*
     * Copyright (c) 2026 Daphne Pfister
     * SPDX-License-Identifier: BSD-2-Clause
     * See LICENSE file for full license text
     */

    use uuid::{NonNilUuid, Uuid};

    use crate::entity::{PanelId, PresenterId};

    use super::*;

    fn nn(b: u8) -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b,
            ]))
        }
    }

    fn p(b: u8) -> PresenterId {
        PresenterId::from_uuid(nn(b))
    }

    fn panel(b: u8) -> PanelId {
        PanelId::from_uuid(nn(b))
    }

    // -----------------------------------------------------------------------
    // Pair operations
    // -----------------------------------------------------------------------

    #[test]
    fn edge_map_add_creates_both_directions() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        assert_eq!(m.by_left(&p(1)), &[panel(10)]);
        assert_eq!(m.by_right(&panel(10)), &[p(1)]);
    }

    #[test]
    fn edge_map_remove_cleans_both_directions() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.add(p(2), panel(10));
        m.remove(&p(1), &panel(10));
        assert_eq!(m.by_left(&p(1)), &[]);
        assert_eq!(m.by_right(&panel(10)), &[p(2)]);
    }

    #[test]
    fn edge_map_remove_last_right_cleans_entry() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.remove(&p(1), &panel(10));
        assert!(!m.contains_left(&p(1)));
        assert!(!m.contains_right(&panel(10)));
        assert!(m.is_empty());
    }

    #[test]
    fn edge_map_contains_correct() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        assert!(m.contains(&p(1), &panel(10)));
        assert!(!m.contains(&p(2), &panel(10)));
        assert!(!m.contains(&p(1), &panel(11)));
    }

    // -----------------------------------------------------------------------
    // Left-keyed operations
    // -----------------------------------------------------------------------

    #[test]
    fn update_by_left_replaces_rights_and_updates_by_right() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.add(p(1), panel(11));
        let removed = m.update_by_left(p(1), &[panel(11), panel(12)]);
        assert_eq!(removed, vec![panel(10)]);
        assert_eq!(m.by_left(&p(1)), &[panel(11), panel(12)]);
        assert_eq!(m.by_right(&panel(10)), &[]);
        assert_eq!(m.by_right(&panel(11)), &[p(1)]);
        assert_eq!(m.by_right(&panel(12)), &[p(1)]);
    }

    #[test]
    fn update_by_left_with_empty_clears_entry() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.update_by_left(p(1), &[]);
        assert!(!m.contains_left(&p(1)));
        assert!(!m.contains_right(&panel(10)));
    }

    #[test]
    fn clear_by_left_removes_all_edges_for_left() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.add(p(1), panel(11));
        m.add(p(2), panel(10));
        m.clear_by_left(&p(1));
        assert!(!m.contains_left(&p(1)));
        assert_eq!(m.by_right(&panel(10)), &[p(2)]);
        assert_eq!(m.by_right(&panel(11)), &[]);
    }

    // -----------------------------------------------------------------------
    // Right-keyed operations
    // -----------------------------------------------------------------------

    #[test]
    fn update_by_right_replaces_lefts_and_updates_by_left() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.add(p(2), panel(10));
        let removed = m.update_by_right(panel(10), &[p(2), p(3)]);
        assert_eq!(removed, vec![p(1)]);
        assert_eq!(m.by_right(&panel(10)), &[p(2), p(3)]);
        assert_eq!(m.by_left(&p(1)), &[]);
        assert_eq!(m.by_left(&p(2)), &[panel(10)]);
        assert_eq!(m.by_left(&p(3)), &[panel(10)]);
    }

    #[test]
    fn update_by_right_with_empty_clears_entry() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.update_by_right(panel(10), &[]);
        assert!(!m.contains_right(&panel(10)));
        assert!(!m.contains_left(&p(1)));
    }

    #[test]
    fn clear_by_right_removes_all_edges_for_right() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.add(p(2), panel(10));
        m.add(p(1), panel(11));
        m.clear_by_right(&panel(10));
        assert!(!m.contains_right(&panel(10)));
        assert_eq!(m.by_left(&p(1)), &[panel(11)]);
        assert_eq!(m.by_left(&p(2)), &[]);
    }

    // -----------------------------------------------------------------------
    // Self-referential (L = R)
    // -----------------------------------------------------------------------

    #[test]
    fn self_referential_group_member() {
        let mut m: EdgeMap<PresenterId, PresenterId> = EdgeMap::new();
        let group = p(1);
        let member1 = p(2);
        let member2 = p(3);
        m.add(group, member1);
        m.add(group, member2);
        // members of group
        assert_eq!(m.by_left(&group), &[member1, member2]);
        // groups of member1
        assert_eq!(m.by_right(&member1), &[group]);
        assert_eq!(m.by_right(&member2), &[group]);
        // remove member2
        m.remove(&group, &member2);
        assert_eq!(m.by_left(&group), &[member1]);
        assert!(m.by_right(&member2).is_empty());
    }

    #[test]
    fn clear_by_right_on_self_referential() {
        let mut m: EdgeMap<PresenterId, PresenterId> = EdgeMap::new();
        let group = p(1);
        let member = p(2);
        m.add(group, member);
        // removing member as a right (member leaves group)
        m.clear_by_right(&member);
        assert!(m.by_left(&group).is_empty());
        assert!(m.by_right(&member).is_empty());
    }

    // -----------------------------------------------------------------------
    // Iterators / misc
    // -----------------------------------------------------------------------

    #[test]
    fn iter_edges_returns_all_pairs() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        m.add(p(1), panel(10));
        m.add(p(1), panel(11));
        m.add(p(2), panel(10));
        let mut pairs: Vec<(PresenterId, PanelId)> = m.iter_edges().collect();
        pairs.sort_by_key(|(l, r)| (l.non_nil_uuid(), r.non_nil_uuid()));
        assert_eq!(
            pairs,
            vec![(p(1), panel(10)), (p(1), panel(11)), (p(2), panel(10))]
        );
    }

    #[test]
    fn len_counts_distinct_lefts() {
        let mut m: EdgeMap<PresenterId, PanelId> = EdgeMap::new();
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
        m.add(p(1), panel(10));
        m.add(p(1), panel(11));
        m.add(p(2), panel(10));
        assert_eq!(m.len(), 2);
        assert!(!m.is_empty());
    }
}
