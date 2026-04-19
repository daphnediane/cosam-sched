/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`Schedule`] — top-level coordination container.
//!
//! Holds all entity storage, relationship edges, and schedule metadata.
//! Fully generic: no entity-type imports here; all typed wiring lives in
//! entity modules.

use crate::edge_map::RawEdgeMap;
use crate::entity::{registered_entity_types, EntityId, EntityType, RuntimeEntityId};
use crate::lookup::{EntityMatcher, MatchPriority};
use crate::value::FieldValue;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use uuid::NonNilUuid;

// ── ScheduleMetadata ──────────────────────────────────────────────────────────

/// Top-level schedule identity and provenance.
#[derive(Debug, Clone)]
pub struct ScheduleMetadata {
    /// Globally unique schedule identity (v7, generated at [`Schedule::new`]).
    pub schedule_id: NonNilUuid,
    /// When this schedule was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Human-readable generator identifier (e.g. `"cosam-convert 0.1"`).
    pub generator: String,
    /// Monotonically increasing edit version counter.
    pub version: u32,
}

// ── Schedule ──────────────────────────────────────────────────────────────────

/// Top-level schedule container.
///
/// - **Entity storage**: `HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>` —
///   one inner map per entity type; indexed by `TypeId::of::<E::InternalData>()`.
/// - **Edge storage**: a single [`RawEdgeMap`] for all relationships.
/// - **Metadata**: schedule UUID, timestamps, generator info.
///
/// There is no separate `EntityStorage` struct; storage lives directly here.
/// Generic `get_internal` / `insert` dispatch via `TypeId`.
#[derive(Debug)]
pub struct Schedule {
    /// Two-level type-erased entity store.
    ///
    /// Outer key: `TypeId::of::<E::InternalData>()`.
    /// Inner key: entity UUID.
    /// Value: `Box<E::InternalData>`.
    entities: HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>,

    /// Single unified edge store for all entity relationships.
    edges: RawEdgeMap,

    /// Schedule identity and provenance.
    pub metadata: ScheduleMetadata,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Create a new, empty schedule with a fresh v7 UUID.
    #[must_use]
    pub fn new() -> Self {
        let raw = uuid::Uuid::now_v7();
        // SAFETY: Uuid::now_v7() is never nil.
        let schedule_id = unsafe { NonNilUuid::new_unchecked(raw) };
        Self {
            entities: HashMap::new(),
            edges: RawEdgeMap::default(),
            metadata: ScheduleMetadata {
                schedule_id,
                created_at: chrono::Utc::now(),
                generator: String::new(),
                version: 0,
            },
        }
    }

    // ── Entity storage ────────────────────────────────────────────────────────

    /// Retrieve a shared reference to an entity's internal data.
    ///
    /// Returns `None` if the entity is not present.
    #[must_use]
    pub fn get_internal<E: EntityType>(&self, id: EntityId<E>) -> Option<&E::InternalData> {
        self.entities
            .get(&TypeId::of::<E::InternalData>())?
            .get(&id.non_nil_uuid())?
            .downcast_ref::<E::InternalData>()
    }

    /// Retrieve a mutable reference to an entity's internal data.
    ///
    /// Returns `None` if the entity is not present.
    pub fn get_internal_mut<E: EntityType>(
        &mut self,
        id: EntityId<E>,
    ) -> Option<&mut E::InternalData> {
        self.entities
            .get_mut(&TypeId::of::<E::InternalData>())?
            .get_mut(&id.non_nil_uuid())?
            .downcast_mut::<E::InternalData>()
    }

    /// Insert or replace an entity's internal data.
    ///
    /// This is the canonical insertion path used by builders and importers.
    pub fn insert<E: EntityType>(&mut self, id: EntityId<E>, data: E::InternalData) {
        self.entities
            .entry(TypeId::of::<E::InternalData>())
            .or_default()
            .insert(id.non_nil_uuid(), Box::new(data));
    }

    /// Remove an entity and clear all of its edge relationships.
    pub fn remove_entity<E: EntityType>(&mut self, id: EntityId<E>) {
        if let Some(map) = self.entities.get_mut(&TypeId::of::<E::InternalData>()) {
            map.remove(&id.non_nil_uuid());
        }
        self.edges.clear_all(id.non_nil_uuid(), E::TYPE_NAME);
    }

    /// Iterate all entities of type `E`, yielding `(EntityId<E>, &E::InternalData)` pairs.
    pub fn iter_entities<E: EntityType>(
        &self,
    ) -> impl Iterator<Item = (EntityId<E>, &E::InternalData)> {
        self.entities
            .get(&TypeId::of::<E::InternalData>())
            .into_iter()
            .flat_map(|map| map.iter())
            .filter_map(|(uuid, boxed)| {
                let data = boxed.downcast_ref::<E::InternalData>()?;
                // SAFETY: uuid came from inserting an EntityId<E>, so it belongs to E.
                let id = unsafe { EntityId::from_uuid(*uuid) };
                Some((id, data))
            })
    }

    /// Count entities of type `E` currently in the schedule.
    #[must_use]
    pub fn entity_count<E: EntityType>(&self) -> usize {
        self.entities
            .get(&TypeId::of::<E::InternalData>())
            .map_or(0, HashMap::len)
    }

    /// Identify which entity type a bare UUID belongs to.
    ///
    /// Queries all inventory-registered entity types (O(5) inner-map lookups).
    /// Returns `None` if the UUID is not found in any type's storage.
    #[must_use]
    pub fn identify(&self, uuid: NonNilUuid) -> Option<RuntimeEntityId> {
        registered_entity_types().find_map(|reg| {
            let inner = self.entities.get(&(reg.type_id)())?;
            if inner.contains_key(&uuid) {
                // SAFETY: we just confirmed uuid is in the inner map for reg.type_name.
                Some(unsafe { RuntimeEntityId::from_uuid(uuid, reg.type_name) })
            } else {
                None
            }
        })
    }

    // ── Edge API ──────────────────────────────────────────────────────────────

    /// All `R` entities reachable from `id` following the L→R direction.
    ///
    /// For het edges: reads `edges[id]` filtered by `R::TYPE_NAME`.
    /// For homo edges (L==R): same — forward edges are stored in `edges`.
    #[must_use]
    pub fn edges_from<L: EntityType, R: EntityType>(&self, id: EntityId<L>) -> Vec<EntityId<R>> {
        self.edges
            .neighbors(id.non_nil_uuid())
            .iter()
            .filter_map(|rid| rid.try_as_typed::<R>())
            .collect()
    }

    /// All `L` entities that have an edge pointing to `id`.
    ///
    /// For het edges: reads `edges[id]` filtered by `L::TYPE_NAME`.
    /// For homo edges (L==R): reads `homogeneous_reverse[id]` filtered by `L::TYPE_NAME`.
    #[must_use]
    pub fn edges_to<L: EntityType, R: EntityType>(&self, id: EntityId<R>) -> Vec<EntityId<L>> {
        let is_homo = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        let source = if is_homo {
            self.edges.homo_reverse(id.non_nil_uuid())
        } else {
            self.edges.neighbors(id.non_nil_uuid())
        };
        source
            .iter()
            .filter_map(|rid| rid.try_as_typed::<L>())
            .collect()
    }

    /// Add an edge from `l` to `r`, using the correct het/homo storage strategy.
    pub fn edge_add<L: EntityType, R: EntityType>(&mut self, l: EntityId<L>, r: EntityId<R>) {
        let is_homo = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        let l_rid = RuntimeEntityId::from_typed(l);
        let r_rid = RuntimeEntityId::from_typed(r);
        if is_homo {
            self.edges.add_homo(l_rid, r_rid);
        } else {
            self.edges.add_het(l_rid, r_rid);
        }
    }

    /// Remove the edge from `l` to `r`.
    pub fn edge_remove<L: EntityType, R: EntityType>(&mut self, l: EntityId<L>, r: EntityId<R>) {
        let is_homo = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        if is_homo {
            self.edges.remove_homo(l.non_nil_uuid(), r.non_nil_uuid());
        } else {
            self.edges.remove_het(l.non_nil_uuid(), r.non_nil_uuid());
        }
    }

    /// Replace all R-type neighbors of `l` with `rights`.
    ///
    /// Removes any existing edges from `l` to entities of type `R`, then
    /// adds edges to each entity in `rights`.
    pub fn edge_set<L: EntityType, R: EntityType>(
        &mut self,
        l: EntityId<L>,
        rights: Vec<EntityId<R>>,
    ) {
        let is_homo = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        let l_rid = RuntimeEntityId::from_typed(l);
        let new_targets: Vec<RuntimeEntityId> = rights
            .iter()
            .map(|r| RuntimeEntityId::from_typed(*r))
            .collect();
        self.edges
            .set_neighbors(l_rid, &new_targets, R::TYPE_NAME, is_homo);
    }

    /// Replace all L-type sources pointing to `r` with `lefts`.
    ///
    /// Used for the reverse (members) direction of homogeneous edges.
    /// Removes each old source's forward edge to `r`, then adds forward edges
    /// from each entity in `lefts` to `r`.
    pub fn edge_set_to<L: EntityType, R: EntityType>(
        &mut self,
        r: EntityId<R>,
        lefts: Vec<EntityId<L>>,
    ) {
        let is_homo = TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>();
        let old_lefts = self.edges_to::<L, R>(r);
        let r_rid = RuntimeEntityId::from_typed(r);
        for l in old_lefts {
            let l_rid = RuntimeEntityId::from_typed(l);
            if is_homo {
                self.edges.remove_homo(l_rid.uuid(), r_rid.uuid());
            } else {
                self.edges.remove_het(l_rid.uuid(), r_rid.uuid());
            }
        }
        for l in lefts {
            let l_rid = RuntimeEntityId::from_typed(l);
            if is_homo {
                self.edges.add_homo(l_rid, r_rid);
            } else {
                self.edges.add_het(l_rid, r_rid);
            }
        }
    }

    // ── Query ─────────────────────────────────────────────────────────────────

    /// Find the best-matching entity of type `E` for a query string.
    ///
    /// Uses `E::match_entity()` against each stored entity.
    /// Returns the entity with the highest [`MatchPriority`], or `None` if
    /// no entity matches.
    #[must_use]
    pub fn find_first<E: EntityMatcher>(&self, query: &str) -> Option<EntityId<E>> {
        let mut best: Option<(EntityId<E>, MatchPriority)> = None;
        for (id, data) in self.iter_entities::<E>() {
            if let Some(priority) = E::match_entity(query, data) {
                let is_better = match &best {
                    None => true,
                    Some((_, best_p)) => priority > *best_p,
                };
                if is_better {
                    best = Some((id, priority));
                }
            }
        }
        best.map(|(id, _)| id)
    }

    /// Find all entities of type `E` matching a query, with their priorities.
    #[must_use]
    pub fn find<E: EntityMatcher>(&self, query: &str) -> Vec<(EntityId<E>, MatchPriority)> {
        let mut results = Vec::new();
        for (id, data) in self.iter_entities::<E>() {
            if let Some(priority) = E::match_entity(query, data) {
                results.push((id, priority));
            }
        }
        results.sort_by(|a: &(EntityId<E>, MatchPriority), b| b.1.cmp(&a.1));
        results
    }
}

// ── Helper: convert Vec<EntityId<E>> to FieldValue ───────────────────────────

/// Convert a `Vec<EntityId<E>>` to a `FieldValue::List` of `EntityIdentifier` items.
///
/// Used by `ReadFn::Schedule` closures in edge-backed field descriptors.
pub fn entity_ids_to_field_value<E: EntityType>(ids: Vec<EntityId<E>>) -> FieldValue {
    use crate::value::FieldValueItem;
    FieldValue::List(
        ids.into_iter()
            .map(|id| FieldValueItem::EntityIdentifier(RuntimeEntityId::from_typed(id)))
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
) -> Result<Vec<EntityId<E>>, crate::value::FieldError> {
    use crate::value::{ConversionError, FieldValueItem};
    match val {
        FieldValue::List(items) => items
            .into_iter()
            .map(|item| match item {
                FieldValueItem::EntityIdentifier(rid) => {
                    rid.try_as_typed::<E>()
                        .ok_or(crate::value::FieldError::Conversion(
                            ConversionError::WrongVariant {
                                expected: E::TYPE_NAME,
                                got: "other entity type",
                            },
                        ))
                }
                _ => Err(crate::value::FieldError::Conversion(
                    ConversionError::WrongVariant {
                        expected: "EntityIdentifier",
                        got: "other",
                    },
                )),
            })
            .collect(),
        _ => Err(crate::value::FieldError::Conversion(
            ConversionError::WrongVariant {
                expected: "List",
                got: "other",
            },
        )),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityId, UuidPreference};
    use crate::event_room::{EventRoomCommonData, EventRoomEntityType, EventRoomInternalData};
    use crate::hotel_room::{HotelRoomCommonData, HotelRoomEntityType, HotelRoomInternalData};
    use crate::panel::{PanelCommonData, PanelEntityType, PanelId, PanelInternalData};
    use crate::panel_type::{PanelTypeCommonData, PanelTypeEntityType, PanelTypeInternalData};
    use crate::panel_uniq_id::PanelUniqId;
    use crate::presenter::{PresenterCommonData, PresenterEntityType, PresenterInternalData};
    use crate::time::TimeRange;

    fn make_panel_type() -> (EntityId<PanelTypeEntityType>, PanelTypeInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = PanelTypeInternalData {
            id,
            data: PanelTypeCommonData {
                prefix: "GP".into(),
                panel_kind: "Guest Panel".into(),
                ..Default::default()
            },
        };
        (id, data)
    }

    fn make_panel() -> (PanelId, PanelInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = PanelInternalData {
            id,
            data: PanelCommonData {
                name: "Test Panel".into(),
                ..Default::default()
            },
            code: PanelUniqId::parse("GP001").unwrap(),
            time_slot: TimeRange::Unspecified,
        };
        (id, data)
    }

    fn make_presenter(name: &str) -> (EntityId<PresenterEntityType>, PresenterInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = PresenterInternalData {
            id,
            data: PresenterCommonData {
                name: name.into(),
                ..Default::default()
            },
        };
        (id, data)
    }

    fn make_event_room(name: &str) -> (EntityId<EventRoomEntityType>, EventRoomInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = EventRoomInternalData {
            id,
            data: EventRoomCommonData {
                room_name: name.into(),
                ..Default::default()
            },
        };
        (id, data)
    }

    fn make_hotel_room(name: &str) -> (EntityId<HotelRoomEntityType>, HotelRoomInternalData) {
        let id = EntityId::from_preference(UuidPreference::GenerateNew);
        let data = HotelRoomInternalData {
            id,
            data: HotelRoomCommonData {
                hotel_room_name: name.into(),
            },
        };
        (id, data)
    }

    // ── Entity storage ────────────────────────────────────────────────────────

    #[test]
    fn insert_and_get_internal() {
        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data.clone());
        let got = sched.get_internal(id).unwrap();
        assert_eq!(got.data.prefix, "GP");
    }

    #[test]
    fn get_internal_missing_returns_none() {
        let sched = Schedule::new();
        let (id, _) = make_panel_type();
        assert!(sched.get_internal(id).is_none());
    }

    #[test]
    fn insert_replaces_existing() {
        let mut sched = Schedule::new();
        let (id, mut data) = make_panel_type();
        sched.insert(id, data.clone());
        data.data.prefix = "SP".into();
        sched.insert(id, data);
        assert_eq!(sched.get_internal(id).unwrap().data.prefix, "SP");
    }

    #[test]
    fn entity_count() {
        let mut sched = Schedule::new();
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
        let (id1, d1) = make_panel_type();
        let (id2, d2) = make_panel_type();
        sched.insert(id1, d1);
        sched.insert(id2, d2);
        assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 2);
    }

    #[test]
    fn iter_entities() {
        let mut sched = Schedule::new();
        let (id1, d1) = make_panel_type();
        let (id2, d2) = make_panel_type();
        sched.insert(id1, d1);
        sched.insert(id2, d2);
        let ids: std::collections::HashSet<_> = sched
            .iter_entities::<PanelTypeEntityType>()
            .map(|(id, _)| id)
            .collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn remove_entity_removes_from_storage() {
        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data);
        assert!(sched.get_internal(id).is_some());
        sched.remove_entity::<PanelTypeEntityType>(id);
        assert!(sched.get_internal(id).is_none());
    }

    // ── Identify ──────────────────────────────────────────────────────────────

    #[test]
    fn identify_returns_correct_type() {
        let mut sched = Schedule::new();
        let (id, data) = make_panel_type();
        sched.insert(id, data);
        let rid = sched.identify(id.non_nil_uuid()).unwrap();
        assert_eq!(rid.type_name(), "panel_type");
        assert_eq!(rid.uuid(), id.non_nil_uuid());
    }

    #[test]
    fn identify_missing_uuid_returns_none() {
        let sched = Schedule::new();
        let (id, _) = make_panel_type();
        assert!(sched.identify(id.non_nil_uuid()).is_none());
    }

    #[test]
    fn identify_distinguishes_types() {
        let mut sched = Schedule::new();
        let (pt_id, pt_data) = make_panel_type();
        let (p_id, p_data) = make_presenter("Alice");
        sched.insert(pt_id, pt_data);
        sched.insert(p_id, p_data);
        let pt_rid = sched.identify(pt_id.non_nil_uuid()).unwrap();
        let p_rid = sched.identify(p_id.non_nil_uuid()).unwrap();
        assert_eq!(pt_rid.type_name(), "panel_type");
        assert_eq!(p_rid.type_name(), "presenter");
    }

    // ── Het edges ─────────────────────────────────────────────────────────────

    #[test]
    fn het_edge_add_and_query_both_directions() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);

        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        let presenters = sched.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        assert_eq!(presenters, vec![pres_id]);

        let panels = sched.edges_from::<PresenterEntityType, PanelEntityType>(pres_id);
        assert_eq!(panels, vec![panel_id]);
    }

    #[test]
    fn het_edge_remove() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);

        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);
        sched.edge_remove::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        assert!(sched
            .edges_from::<PanelEntityType, PresenterEntityType>(panel_id)
            .is_empty());
        assert!(sched
            .edges_from::<PresenterEntityType, PanelEntityType>(pres_id)
            .is_empty());
    }

    #[test]
    fn het_edge_set_replaces_all() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (p1_id, p1_data) = make_presenter("Alice");
        let (p2_id, p2_data) = make_presenter("Bob");
        let (p3_id, p3_data) = make_presenter("Carol");
        sched.insert(panel_id, panel_data);
        sched.insert(p1_id, p1_data);
        sched.insert(p2_id, p2_data);
        sched.insert(p3_id, p3_data);

        sched.edge_set::<PanelEntityType, PresenterEntityType>(panel_id, vec![p1_id, p2_id]);
        let mut presenters = sched.edges_from::<PanelEntityType, PresenterEntityType>(panel_id);
        presenters.sort_by_key(|id| id.uuid());
        let mut expected = vec![p1_id, p2_id];
        expected.sort_by_key(|id| id.uuid());
        assert_eq!(presenters, expected);

        sched.edge_set::<PanelEntityType, PresenterEntityType>(panel_id, vec![p3_id]);
        assert_eq!(
            sched.edges_from::<PanelEntityType, PresenterEntityType>(panel_id),
            vec![p3_id]
        );
        // p1 and p2 no longer link back to panel
        assert!(sched
            .edges_from::<PresenterEntityType, PanelEntityType>(p1_id)
            .is_empty());
        assert!(sched
            .edges_from::<PresenterEntityType, PanelEntityType>(p2_id)
            .is_empty());
    }

    #[test]
    fn remove_entity_clears_het_edges() {
        let mut sched = Schedule::new();
        let (panel_id, panel_data) = make_panel();
        let (pres_id, pres_data) = make_presenter("Alice");
        sched.insert(panel_id, panel_data);
        sched.insert(pres_id, pres_data);
        sched.edge_add::<PanelEntityType, PresenterEntityType>(panel_id, pres_id);

        sched.remove_entity::<PanelEntityType>(panel_id);

        // Edge from presenter side should be gone too
        assert!(sched
            .edges_from::<PresenterEntityType, PanelEntityType>(pres_id)
            .is_empty());
    }

    // ── EventRoom / HotelRoom het edges ───────────────────────────────────────

    #[test]
    fn event_room_hotel_room_het_edge() {
        let mut sched = Schedule::new();
        let (room_id, room_data) = make_event_room("Panel 1");
        let (hotel_id, hotel_data) = make_hotel_room("East Hall");
        sched.insert(room_id, room_data);
        sched.insert(hotel_id, hotel_data);

        sched.edge_add::<EventRoomEntityType, HotelRoomEntityType>(room_id, hotel_id);

        let hotels = sched.edges_from::<EventRoomEntityType, HotelRoomEntityType>(room_id);
        assert_eq!(hotels, vec![hotel_id]);

        // Reverse: hotel_room.event_rooms via edges_from::<HotelRoom, EventRoom>
        let rooms = sched.edges_from::<HotelRoomEntityType, EventRoomEntityType>(hotel_id);
        assert_eq!(rooms, vec![room_id]);
    }

    // ── Homo edges (Presenter → Presenter) ───────────────────────────────────

    #[test]
    fn homo_edge_groups_and_members() {
        let mut sched = Schedule::new();
        let (member_id, member_data) = make_presenter("Alice");
        let (group_id, group_data) = make_presenter("The Group");
        sched.insert(member_id, member_data);
        sched.insert(group_id, group_data);

        // member → group (forward homo edge: member is in group)
        sched.edge_add::<PresenterEntityType, PresenterEntityType>(member_id, group_id);

        // groups of member: edges_from(member)
        let groups = sched.edges_from::<PresenterEntityType, PresenterEntityType>(member_id);
        assert_eq!(groups, vec![group_id]);

        // members of group: edges_to(group)
        let members = sched.edges_to::<PresenterEntityType, PresenterEntityType>(group_id);
        assert_eq!(members, vec![member_id]);
    }

    #[test]
    fn homo_edge_remove() {
        let mut sched = Schedule::new();
        let (member_id, member_data) = make_presenter("Alice");
        let (group_id, group_data) = make_presenter("The Group");
        sched.insert(member_id, member_data);
        sched.insert(group_id, group_data);

        sched.edge_add::<PresenterEntityType, PresenterEntityType>(member_id, group_id);
        sched.edge_remove::<PresenterEntityType, PresenterEntityType>(member_id, group_id);

        assert!(sched
            .edges_from::<PresenterEntityType, PresenterEntityType>(member_id)
            .is_empty());
        assert!(sched
            .edges_to::<PresenterEntityType, PresenterEntityType>(group_id)
            .is_empty());
    }

    #[test]
    fn homo_edge_set_replaces() {
        let mut sched = Schedule::new();
        let (member_id, member_data) = make_presenter("Alice");
        let (g1_id, g1_data) = make_presenter("Group A");
        let (g2_id, g2_data) = make_presenter("Group B");
        sched.insert(member_id, member_data);
        sched.insert(g1_id, g1_data);
        sched.insert(g2_id, g2_data);

        sched.edge_set::<PresenterEntityType, PresenterEntityType>(member_id, vec![g1_id]);
        assert_eq!(
            sched.edges_from::<PresenterEntityType, PresenterEntityType>(member_id),
            vec![g1_id]
        );

        sched.edge_set::<PresenterEntityType, PresenterEntityType>(member_id, vec![g2_id]);
        assert_eq!(
            sched.edges_from::<PresenterEntityType, PresenterEntityType>(member_id),
            vec![g2_id]
        );
        assert!(sched
            .edges_to::<PresenterEntityType, PresenterEntityType>(g1_id)
            .is_empty());
    }

    #[test]
    fn edge_set_to_sets_members() {
        let mut sched = Schedule::new();
        let (m1_id, m1_data) = make_presenter("Alice");
        let (m2_id, m2_data) = make_presenter("Bob");
        let (g_id, g_data) = make_presenter("The Group");
        sched.insert(m1_id, m1_data);
        sched.insert(m2_id, m2_data);
        sched.insert(g_id, g_data);

        // Set members of group to [m1, m2]
        sched.edge_set_to::<PresenterEntityType, PresenterEntityType>(g_id, vec![m1_id, m2_id]);

        let mut members = sched.edges_to::<PresenterEntityType, PresenterEntityType>(g_id);
        members.sort_by_key(|id| id.uuid());
        let mut expected = vec![m1_id, m2_id];
        expected.sort_by_key(|id| id.uuid());
        assert_eq!(members, expected);

        // m1 and m2 should have group in their groups list
        assert_eq!(
            sched.edges_from::<PresenterEntityType, PresenterEntityType>(m1_id),
            vec![g_id]
        );
        assert_eq!(
            sched.edges_from::<PresenterEntityType, PresenterEntityType>(m2_id),
            vec![g_id]
        );

        // Replace with just m1
        sched.edge_set_to::<PresenterEntityType, PresenterEntityType>(g_id, vec![m1_id]);
        assert_eq!(
            sched.edges_to::<PresenterEntityType, PresenterEntityType>(g_id),
            vec![m1_id]
        );
        assert!(sched
            .edges_from::<PresenterEntityType, PresenterEntityType>(m2_id)
            .is_empty());
    }

    #[test]
    fn remove_entity_clears_homo_edges() {
        let mut sched = Schedule::new();
        let (member_id, member_data) = make_presenter("Alice");
        let (group_id, group_data) = make_presenter("The Group");
        sched.insert(member_id, member_data);
        sched.insert(group_id, group_data);
        sched.edge_add::<PresenterEntityType, PresenterEntityType>(member_id, group_id);

        sched.remove_entity::<PresenterEntityType>(member_id);

        // group should no longer see member
        assert!(sched
            .edges_to::<PresenterEntityType, PresenterEntityType>(group_id)
            .is_empty());
    }

    // ── find_first ────────────────────────────────────────────────────────────

    #[test]
    fn find_first_matches_by_name() {
        let mut sched = Schedule::new();
        let (id, data) = make_presenter("Alice Example");
        sched.insert(id, data);
        let found = sched.find_first::<PresenterEntityType>("alice");
        assert_eq!(found, Some(id));
    }

    #[test]
    fn find_first_returns_none_for_no_match() {
        let mut sched = Schedule::new();
        let (id, data) = make_presenter("Alice");
        sched.insert(id, data);
        assert!(sched.find_first::<PresenterEntityType>("bob").is_none());
    }

    #[test]
    fn find_first_prefers_exact_over_prefix() {
        let mut sched = Schedule::new();
        let (prefix_id, prefix_data) = make_presenter("Ali");
        let (exact_id, exact_data) = make_presenter("alice");
        sched.insert(prefix_id, prefix_data);
        sched.insert(exact_id, exact_data);
        let found = sched.find_first::<PresenterEntityType>("alice");
        assert_eq!(found, Some(exact_id));
    }

    // ── entity_ids_to_field_value / field_value_to_entity_ids ─────────────────

    #[test]
    fn entity_ids_roundtrip_through_field_value() {
        let (id1, _) = make_presenter("Alice");
        let (id2, _) = make_presenter("Bob");
        let ids = vec![id1, id2];
        let fv = entity_ids_to_field_value(ids.clone());
        let back = field_value_to_entity_ids::<PresenterEntityType>(fv).unwrap();
        assert_eq!(back, ids);
    }

    #[test]
    fn field_value_to_entity_ids_wrong_type_is_error() {
        let (room_id, _) = make_event_room("Panel 1");
        let fv = entity_ids_to_field_value(vec![room_id]);
        let result = field_value_to_entity_ids::<PresenterEntityType>(fv);
        assert!(result.is_err());
    }
}
