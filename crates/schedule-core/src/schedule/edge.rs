/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge operations for [`Schedule`].

use crate::edge::cache::TransitiveEdgeCache;
use crate::edge::map::EdgeError;
use crate::edge::{FullEdge, HalfEdge};
use crate::entity::{DynamicEntityId, EntityType, EntityUuid};
use crate::value::{ConversionError, FieldError, FieldValue, FieldValueItem};
use crate::EntityId;
use uuid::NonNilUuid;

use super::Schedule;

impl Schedule {
    // ── Edge API ──────────────────────────────────────────────────────────────

    /// All neighbor field node IDs reachable via the given field node, filtered by far-side field.
    ///
    /// Lowest-level field-based query; suitable for entity-module read functions
    /// that have a concrete [`FullEdge`] in hand.
    /// Only returns connections where the far-side field matches `far_field`.
    /// Returns the full [`crate::entity::RuntimeEntityId`] of each neighbor (including both
    /// the entity UUID and the field ID of the reverse edge).
    #[must_use]
    pub fn connected_field_nodes(
        &self,
        node: impl DynamicEntityId,
        edge: FullEdge,
    ) -> Vec<crate::entity::RuntimeEntityId> {
        self.edges.neighbors(node, edge)
    }

    /// Returns all entities of type `R` connected to `node` via the given far field.
    ///
    /// The field node ID specifies which field on the entity stores the relationship,
    /// useful when an entity has multiple fields relating to the same target type.
    /// The far field parameter specifies which field on the target entity stores the reverse relationship.
    #[must_use]
    pub fn connected_entities<R: EntityType>(
        &self,
        node: impl DynamicEntityId,
        edge: FullEdge,
    ) -> Vec<EntityId<R>> {
        self.connected_field_nodes(node, edge)
            .iter()
            // SAFETY: edge_map validates that all stored entities match edge.far.entity_type_name()
            .map(|fn_id| unsafe { EntityId::new_unchecked(fn_id.entity_uuid()) })
            .collect()
    }

    /// All `Far` entities reachable from `near` via edges.
    ///
    /// When `Near` and `Far` are the same entity type (homogeneous edge),
    /// follows edges transitively via the cache (e.g.
    /// `inclusive_edges(alice_members, &FIELD_GROUPS)` returns all groups
    /// alice belongs to, transitively — not alice herself).
    /// For heterogeneous edges: single-hop lookup via `connected_field_nodes`.
    ///
    /// Takes `&self`; the edge cache is updated through interior mutability.
    #[must_use]
    pub fn inclusive_edges<Near: EntityType, Far: EntityType>(
        &self,
        near: EntityId<Near>,
        edge: FullEdge,
    ) -> Vec<EntityId<Far>> {
        if Near::TYPE_NAME == Far::TYPE_NAME {
            let uuids = {
                let mut cache_opt = self.transitive_edge_cache.borrow_mut();
                let cache = cache_opt.get_or_insert_with(TransitiveEdgeCache::default);
                cache.get_or_compute(&self.edges, near, edge)
            };
            uuids
                .into_iter()
                // SAFETY: uuid came from the edge map which only stores valid entity IDs of type Far.
                .map(|uuid| unsafe { EntityId::new_unchecked(uuid) })
                .collect()
        } else {
            self.connected_field_nodes(near, edge)
                .into_iter()
                // SAFETY: The field descriptor ensures the UUID belongs to entity type Far.
                .map(|fn_id| unsafe { EntityId::<Far>::new_unchecked(fn_id.entity_uuid()) })
                .collect()
        }
    }

    /// Add edges from `near` to each node in `far_nodes`.
    ///
    /// Adds the edges to [`crate::edge_map::RawEdgeMap`]. When the edge is transitive,
    /// also invalidates the [`TransitiveEdgeCache`].
    ///
    /// After updating the cache, if the mirror is enabled each new endpoint
    /// is incrementally `insert`ed into the canonical owner's list field
    /// (via [`crate::crdt::edge::list_append_unique`]) — **not** rewritten in
    /// full — so concurrent add/add from two replicas converges to the
    /// union rather than LWW on the list object.
    ///
    /// Returns the UUIDs of edges that were actually added (excluding duplicates).
    pub fn edge_add(
        &mut self,
        near: impl DynamicEntityId,
        edge: FullEdge,
        far_nodes: impl IntoIterator<Item = impl DynamicEntityId>,
    ) -> Result<Vec<NonNilUuid>, EdgeError> {
        let near_type = near.entity_type_name();
        let far_type = edge.far.entity_type_name();

        let added = self.edges.add_edge(near, edge, far_nodes)?;

        // Invalidate transitive cache when the two endpoints share the same entity type.
        if near_type == far_type {
            *self.transitive_edge_cache.borrow_mut() = None;
        }

        // CRDT mirror
        self.mirror_edge_add(&near, edge, &added);

        Ok(added)
    }

    /// Remove edges from `near` to each node in `far_nodes`.
    ///
    /// The CRDT mirror uses an incremental delete on observed indices so
    /// concurrent add-vs-unobserved-remove resolves add-wins.
    ///
    /// Returns the UUIDs of edges that were actually removed.
    pub fn edge_remove(
        &mut self,
        near: impl DynamicEntityId,
        edge: FullEdge,
        far_nodes: impl IntoIterator<Item = impl DynamicEntityId>,
    ) -> Vec<NonNilUuid> {
        let near_type = near.entity_type_name();
        let far_type = edge.far.entity_type_name();

        let removed = self.edges.remove_edge(near, edge, far_nodes);

        // Invalidate transitive cache when the two endpoints share the same entity type.
        if near_type == far_type {
            *self.transitive_edge_cache.borrow_mut() = None;
        }

        // CRDT mirror
        self.mirror_edge_remove(&near, edge, &removed);

        removed
    }

    /// Replace all far-side neighbors of `near` with `targets`.
    ///
    /// `near` identifies the entity; `edge` specifies both near and far fields.
    /// Works from either direction — `set_neighbors` handles the bidirectional bookkeeping.
    ///
    /// When the two endpoints share the same entity type (transitive/homogeneous edge),
    /// the transitive edge cache is invalidated.
    ///
    /// Returns the number of edges added and removed.
    pub fn edge_set(
        &mut self,
        near: impl DynamicEntityId,
        edge: FullEdge,
        targets: impl IntoIterator<Item = impl DynamicEntityId>,
    ) -> Result<(usize, usize), EdgeError> {
        let (added, removed) = self.edges.set_neighbors(near, edge, targets)?;

        // Invalidate transitive cache when near and far share the same entity type.
        if near.entity_type_name() == edge.far.entity_type_name() {
            *self.transitive_edge_cache.borrow_mut() = None;
        }

        // CRDT mirror
        self.mirror_edge_set(&near, edge, &added, &removed);

        Ok((added.len(), removed.len()))
    }

    /// After `edge_add`, incrementally append each new endpoint into the
    /// canonical owner's list field. Concurrent add/add converges to the
    /// union because both replicas insert into the same shared list
    /// [`ObjId`](automerge::ObjId) created up-front by
    /// `mirror_entity_fields` (via `ensure_owner_list` on each `Owner` field).
    ///
    /// Ownership is resolved from the field descriptors embedded in `edge`
    /// via [`crate::crdt::edge::canonical_owner`].
    fn mirror_edge_add(
        &mut self,
        near: &impl DynamicEntityId,
        edge: FullEdge,
        far_uuids: &[NonNilUuid],
    ) {
        if !self.mirror_enabled {
            return;
        }
        let Some(canon) = crate::crdt::edge::canonical_owner(edge.near, edge.far) else {
            return;
        };
        let near_uuid = near.entity_uuid();
        for far_uuid in far_uuids {
            let (owner_uuid, target_uuid) = if canon.near_is_owner {
                (near_uuid, *far_uuid)
            } else {
                (*far_uuid, near_uuid)
            };
            if let Err(e) = crate::crdt::edge::list_append_unique(
                &mut self.doc,
                canon.owner_type(),
                owner_uuid,
                canon.target_type(),
                canon.field_name(),
                target_uuid,
            ) {
                debug_assert!(false, "CRDT edge_add mirror failed: {e}");
                let _ = e;
            }
        }
    }

    /// After `edge_remove`, incrementally delete every occurrence of each
    /// endpoint from the canonical owner's list.  Concurrent add-vs-
    /// unobserved-remove resolves add-wins: the remove only targets
    /// indices this actor observed, so an insert recorded on a parallel
    /// branch survives the merge.
    ///
    /// Ownership is resolved from the field descriptors embedded in `edge`
    /// via [`crate::crdt::edge::canonical_owner`].
    fn mirror_edge_remove(
        &mut self,
        near: &impl DynamicEntityId,
        edge: FullEdge,
        far_uuids: &[NonNilUuid],
    ) {
        if !self.mirror_enabled {
            return;
        }
        let Some(canon) = crate::crdt::edge::canonical_owner(edge.near, edge.far) else {
            return;
        };
        let near_uuid = near.entity_uuid();
        for far_uuid in far_uuids {
            let (owner_uuid, target_uuid) = if canon.near_is_owner {
                (near_uuid, *far_uuid)
            } else {
                (*far_uuid, near_uuid)
            };
            if let Err(e) = crate::crdt::edge::list_remove_uuid(
                &mut self.doc,
                canon.owner_type(),
                owner_uuid,
                canon.target_type(),
                canon.field_name(),
                target_uuid,
            ) {
                debug_assert!(false, "CRDT edge_remove mirror failed: {e}");
                let _ = e;
            }
        }
    }

    /// Edge-set mirror — incremental version.
    ///
    /// Applies the `(added, removed)` diff produced by [`EdgeMap::set_neighbors`]
    /// as a series of per-edge incremental CRDT operations (`list_append_unique`
    /// / `list_remove_uuid`) so that concurrent edits on other replicas are
    /// preserved rather than clobbered by a full list rewrite.
    ///
    /// When near is the canonical owner, all writes target near's list.
    /// When far is the canonical owner, each added/removed far entity's list
    /// is updated to add/remove near.
    ///
    /// Field refs are derived directly from the [`crate::crdt::edge::CanonicalOwner`]
    /// returned by [`crate::crdt::edge::canonical_owner`].
    fn mirror_edge_set(
        &mut self,
        near: &impl DynamicEntityId,
        edge: FullEdge,
        added: &[NonNilUuid],
        removed: &[NonNilUuid],
    ) {
        if !self.mirror_enabled {
            return;
        }
        let Some(canon) = crate::crdt::edge::canonical_owner(edge.near, edge.far) else {
            return;
        };
        let near_uuid = near.entity_uuid();
        if canon.near_is_owner {
            // Apply diffs to near's owner list.
            for target_uuid in added {
                if let Err(e) = crate::crdt::edge::list_append_unique(
                    &mut self.doc,
                    canon.owner_type(),
                    near_uuid,
                    canon.target_type(),
                    canon.field_name(),
                    *target_uuid,
                ) {
                    debug_assert!(false, "CRDT edge_set mirror (append) failed: {e}");
                    let _ = e;
                }
            }
            for target_uuid in removed {
                if let Err(e) = crate::crdt::edge::list_remove_uuid(
                    &mut self.doc,
                    canon.owner_type(),
                    near_uuid,
                    canon.target_type(),
                    canon.field_name(),
                    *target_uuid,
                ) {
                    debug_assert!(false, "CRDT edge_set mirror (remove) failed: {e}");
                    let _ = e;
                }
            }
            return;
        }
        // Far is owner — for each added far, append near to that far's list.
        // For each removed far, remove near from that far's list.
        for far_uuid in added {
            if let Err(e) = crate::crdt::edge::list_append_unique(
                &mut self.doc,
                canon.owner_type(),
                *far_uuid,
                canon.target_type(),
                canon.field_name(),
                near_uuid,
            ) {
                debug_assert!(false, "CRDT edge_set mirror (append) failed: {e}");
                let _ = e;
            }
        }
        for far_uuid in removed {
            if let Err(e) = crate::crdt::edge::list_remove_uuid(
                &mut self.doc,
                canon.owner_type(),
                *far_uuid,
                canon.target_type(),
                canon.field_name(),
                near_uuid,
            ) {
                debug_assert!(false, "CRDT edge_set mirror (remove) failed: {e}");
                let _ = e;
            }
        }
    }
}

// ── Helper: convert Vec<EntityId<E>> to FieldValue ───────────────────────────

/// Convert a `Vec<EntityId<E>>` to a `FieldValue::List` of `EntityIdentifier` items.
///
/// Used by `ReadFn::Schedule` closures in edge-backed field descriptors.
pub fn entity_ids_to_field_value<E: EntityType>(ids: Vec<EntityId<E>>) -> FieldValue {
    FieldValue::List(
        ids.into_iter()
            .map(|id| FieldValueItem::EntityIdentifier(id.into()))
            .collect(),
    )
}

/// Parse a `FieldValue` into a `Vec<EntityId<E>>`.
///
/// Accepts `FieldValue::List(...)` of `EntityIdentifier` items; returns
/// `Err(FieldError::Conversion(...))` for any non-matching items or variants.
///
/// Used by `WriteFn::Schedule` closures in edge-backed field descriptors.
pub fn field_value_to_entity_ids<E: EntityType>(
    val: FieldValue,
) -> Result<Vec<EntityId<E>>, FieldError> {
    match val {
        FieldValue::List(items) => items
            .into_iter()
            .map(|item| match item {
                FieldValueItem::EntityIdentifier(rid) => rid.try_into().map_err(|_| {
                    FieldError::Conversion(ConversionError::WrongVariant {
                        expected: E::TYPE_NAME,
                        got: "other entity type",
                    })
                }),
                _ => Err(FieldError::Conversion(ConversionError::WrongVariant {
                    expected: "EntityIdentifier",
                    got: "other",
                })),
            })
            .collect(),
        _ => Err(FieldError::Conversion(ConversionError::WrongVariant {
            expected: "List",
            got: "other",
        })),
    }
}

/// Parse a `FieldValue` into a `Vec<RuntimeEntityId>`.
///
/// Accepts `FieldValue::List(...)` of `EntityIdentifier` items; returns
/// `Err(FieldError::Conversion(...))` for any non-matching items or variants.
///
/// Used by edge write operations where the target entity type is not known at compile time.
pub fn field_value_to_runtime_entity_ids(
    val: FieldValue,
) -> Result<Vec<crate::entity::RuntimeEntityId>, FieldError> {
    match val {
        FieldValue::List(items) => items
            .into_iter()
            .map(|item| match item {
                FieldValueItem::EntityIdentifier(rid) => Ok(rid),
                _ => Err(FieldError::Conversion(ConversionError::WrongVariant {
                    expected: "EntityIdentifier",
                    got: "other",
                })),
            })
            .collect(),
        _ => Err(FieldError::Conversion(ConversionError::WrongVariant {
            expected: "List",
            got: "other",
        })),
    }
}

/// Read entities connected via a field's own edge relationship.
///
/// Determines the far field from the edge kind and queries connected entities.
/// This is a convenience method for use by both `EdgeDescriptor` and `FieldDescriptor`.
pub fn read_edge<E: EntityType>(
    schedule: &Schedule,
    id: EntityId<E>,
    field: &'static dyn HalfEdge,
) -> Result<Option<FieldValue>, FieldError> {
    use crate::edge::EdgeKind;
    match field.edge_kind() {
        EdgeKind::Owner { target_field, .. } => {
            // Construct a FullEdge from field (near) and target_field (far)
            let edge = crate::edge::FullEdge {
                near: field,
                far: *target_field,
            };
            read_full_edge(schedule, id, &edge)
        }
        EdgeKind::Target { source_fields } => {
            // For target fields with a single source, treat it like Owner
            match source_fields {
                [single] => {
                    // Construct a FullEdge from field (near) and single source (far)
                    let edge = crate::edge::FullEdge {
                        near: field,
                        far: *single,
                    };
                    read_full_edge(schedule, id, &edge)
                }
                _ => {
                    // For multiple source fields, construct FullEdges and combine
                    let edges: Vec<crate::edge::FullEdge> = source_fields
                        .iter()
                        .map(|source| {
                            // SAFETY: source is a &'static HalfEdge (edge descriptors are static singletons).
                            let static_source: &'static dyn HalfEdge =
                                unsafe { std::mem::transmute(*source) };
                            crate::edge::FullEdge {
                                near: field,
                                far: static_source,
                            }
                        })
                        .collect();
                    let edge_refs: Vec<&crate::edge::FullEdge> = edges.iter().collect();
                    combine_full_edges(schedule, id, &edge_refs)
                }
            }
        }
        EdgeKind::NonEdge => Ok(Some(FieldValue::List(vec![]))),
    }
}

/// Read entities connected via a single [`FullEdge`].
///
/// Queries connected entities from the near entity through the near field to the far field.
/// This is a convenience method for use by both `EdgeDescriptor` and `FieldDescriptor`.
pub fn read_full_edge<E: EntityType>(
    schedule: &Schedule,
    id: EntityId<E>,
    edge: &crate::edge::FullEdge,
) -> Result<Option<FieldValue>, FieldError> {
    let neighbors = schedule.connected_field_nodes(id, *edge);
    let items = neighbors
        .into_iter()
        .map(FieldValueItem::EntityIdentifier)
        .collect();
    Ok(Some(FieldValue::List(items)))
}

/// Read entities connected via multiple [`FullEdge`]s and combine the results.
///
/// Queries each edge and unions the results with deduplication by UUID.
/// This is a convenience method for use by both `EdgeDescriptor` and `FieldDescriptor`.
pub fn combine_full_edges<E: EntityType>(
    schedule: &Schedule,
    id: EntityId<E>,
    edges: &[&crate::edge::FullEdge],
) -> Result<Option<FieldValue>, FieldError> {
    let mut all_ids = Vec::new();
    for edge in edges {
        let neighbors = schedule.connected_field_nodes(id, **edge);
        all_ids.extend(neighbors);
    }
    // Deduplicate by UUID
    all_ids.sort_by_key(|e| e.entity_uuid());
    all_ids.dedup_by_key(|e| e.entity_uuid());
    let items = all_ids
        .into_iter()
        .map(FieldValueItem::EntityIdentifier)
        .collect();
    Ok(Some(FieldValue::List(items)))
}

/// Add entities to an edge relationship via a [`FullEdge`].
///
/// Adds the target entities from the value to the edge without removing existing ones.
/// If `exclusive_with` is provided, removes each target from that exclusive sibling field first.
/// This is a convenience method for use by both `EdgeDescriptor` and `FieldDescriptor`.
pub fn add_edge<E: EntityType>(
    schedule: &mut Schedule,
    id: EntityId<E>,
    edge: &crate::edge::FullEdge,
    exclusive_with: Option<&crate::edge::FullEdge>,
    value: FieldValue,
) -> Result<(), FieldError> {
    let target_ids = field_value_to_runtime_entity_ids(value)?;
    // Add edges first, then clean up exclusive_with for only the actually-added targets
    let added = schedule.edge_add(id, *edge, target_ids)?;
    if let Some(exclusive_edge) = exclusive_with {
        // SAFETY: The added UUIDs are already validated to be edge.far.entity_type_name().
        // If exclusive_edge.far.entity_type_name() is the same, we can use the same UUIDs.
        // If different, the edge isn't really exclusive (different types).
        let added_runtime: Vec<crate::entity::RuntimeEntityId> = added
            .into_iter()
            .map(|uuid| unsafe {
                crate::entity::RuntimeEntityId::new_unchecked(uuid, edge.far.entity_type_name())
            })
            .collect();
        let _ = schedule.edge_remove(id, *exclusive_edge, added_runtime);
    }
    Ok(())
}

/// Remove entities from an edge relationship via a [`FullEdge`].
///
/// Removes the target entities from the value from the edge.
/// If `exclusive_with` is provided, also removes from that exclusive sibling field.
/// This is a convenience method for use by both `EdgeDescriptor` and `FieldDescriptor`.
pub fn remove_edge<E: EntityType>(
    schedule: &mut Schedule,
    id: EntityId<E>,
    edge: &crate::edge::FullEdge,
    exclusive_with: Option<&crate::edge::FullEdge>,
    value: FieldValue,
) -> Result<(), FieldError> {
    let target_ids = field_value_to_runtime_entity_ids(value)?;
    // Remove edges first, then clean up exclusive_with for only the actually-removed targets
    let removed = schedule.edge_remove(id, *edge, target_ids);
    if let Some(exclusive_edge) = exclusive_with {
        // SAFETY: The removed UUIDs are already validated to be edge.far.entity_type_name().
        // If exclusive_edge.far.entity_type_name() is the same, we can use the same UUIDs.
        // If different, the edge isn't really exclusive (different types).
        let removed_runtime: Vec<crate::entity::RuntimeEntityId> = removed
            .into_iter()
            .map(|uuid| unsafe {
                crate::entity::RuntimeEntityId::new_unchecked(uuid, edge.far.entity_type_name())
            })
            .collect();
        let _ = schedule.edge_remove(id, *exclusive_edge, removed_runtime);
    }
    Ok(())
}

/// Set the edges from an entity to target entities via a field.
///
/// Handles `exclusive_with` by removing each target from the exclusive sibling field.
/// This is a convenience method for use by both `EdgeDescriptor` and `FieldDescriptor`.
pub fn write_edge<E: EntityType>(
    schedule: &mut Schedule,
    id: EntityId<E>,
    field: &'static dyn HalfEdge,
    value: FieldValue,
) -> Result<(), FieldError> {
    use crate::edge::EdgeKind;
    let target_ids = field_value_to_runtime_entity_ids(value)?;
    let (far_field, exclusive_with) = match field.edge_kind() {
        EdgeKind::Owner {
            target_field,
            exclusive_with,
        } => (*target_field, *exclusive_with),
        EdgeKind::Target { source_fields } => {
            // For target fields, write to the first source field
            match source_fields {
                [single] => (*single, None),
                _ => {
                    // Multiple sources - return error for now
                    return Err(FieldError::Conversion(ConversionError::InvalidEdge {
                        reason: "Multiple source fields not supported for write_edge".to_string(),
                    }));
                }
            }
        }
        EdgeKind::NonEdge => {
            return Err(FieldError::Conversion(ConversionError::InvalidEdge {
                reason: "NonEdge fields cannot use write_edge".to_string(),
            }));
        }
    };

    // Construct FullEdge for edge_set
    let edge = crate::edge::FullEdge {
        near: field,
        far: far_field,
    };

    // If there's an exclusive sibling field, remove edges from it first
    if let Some(exclusive_field) = exclusive_with {
        let exclusive_far: &'static dyn HalfEdge = match exclusive_field.edge_kind() {
            EdgeKind::Owner { target_field, .. } => *target_field,
            EdgeKind::Target { source_fields } => match source_fields {
                [single] => *single,
                _ => {
                    return Err(FieldError::Conversion(ConversionError::InvalidEdge {
                        reason: "Exclusive field has multiple sources".to_string(),
                    }));
                }
            },
            EdgeKind::NonEdge => {
                return Err(FieldError::Conversion(ConversionError::InvalidEdge {
                    reason: "Exclusive field is NonEdge".to_string(),
                }));
            }
        };
        let exclusive_edge = crate::edge::FullEdge {
            near: exclusive_field,
            far: exclusive_far,
        };
        // Remove each target entity from the exclusive sibling field
        for target_id in &target_ids {
            let _ = schedule.edge_remove(id, exclusive_edge, std::iter::once(*target_id));
        }
    }

    schedule.edge_set(id, edge, target_ids)?;
    Ok(())
}
