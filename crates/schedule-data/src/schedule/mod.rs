/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule container with entity storage, edge indexing, and UUID registry.

pub mod edge_index;
mod metadata;
mod storage;

pub use edge_index::EdgeIndex;
pub use metadata::{GeneratorInfo, ScheduleMetadata};
pub use storage::{
    BuildError, EntityStorage, EntityStore, InsertError, TypedEdgeStorage, TypedStorage,
};

use crate::entity::{
    DirectedEdge, EntityKind, EntityType, EntityUUID, EventRoomId, EventRoomToHotelRoomEntityType,
    EventRoomToHotelRoomId, HotelRoomId, InternalData, PanelId, PanelToEventRoomEntityType,
    PanelToEventRoomId, PanelToPanelTypeEntityType, PanelToPanelTypeId, PanelToPresenterEntityType,
    PanelToPresenterId, PanelTypeId, PresenterId, PresenterToGroupEntityType, PresenterToGroupId,
    TypedId,
};
use std::collections::HashMap;
use uuid::NonNilUuid;

/// Central schedule container.
///
/// Holds all entities, relationships, metadata, and provides a unified API
/// for schedule operations.
#[derive(Debug)]
pub struct Schedule {
    /// Entity storage for all entity types.
    pub entities: EntityStorage,

    /// UUID registry mapping UUIDs to their entity kind.
    /// @todo: This should probably be part of EntityStorage
    uuid_registry: HashMap<NonNilUuid, EntityKind>,

    /// Schedule metadata.
    metadata: ScheduleMetadata,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Create a new empty schedule.
    pub fn new() -> Self {
        Self {
            entities: EntityStorage::new(),
            uuid_registry: HashMap::new(),
            metadata: ScheduleMetadata::default(),
        }
    }

    /// Get the schedule metadata.
    pub fn metadata(&self) -> &ScheduleMetadata {
        &self.metadata
    }

    /// Get mutable schedule metadata.
    pub fn metadata_mut(&mut self) -> &mut ScheduleMetadata {
        &mut self.metadata
    }

    // -----------------------------------------------------------------------
    // UUID registry and identification
    // -----------------------------------------------------------------------

    /// Identify which entity kind a UUID belongs to.
    pub fn identify(&self, uuid: NonNilUuid) -> Option<EntityUUID> {
        let kind = self.uuid_registry.get(&uuid)?;
        match kind {
            EntityKind::Panel => Some(EntityUUID::Panel(PanelId::from_uuid(uuid))),
            EntityKind::Presenter => Some(EntityUUID::Presenter(PresenterId::from_uuid(uuid))),
            EntityKind::EventRoom => Some(EntityUUID::EventRoom(EventRoomId::from_uuid(uuid))),
            EntityKind::HotelRoom => Some(EntityUUID::HotelRoom(HotelRoomId::from_uuid(uuid))),
            EntityKind::PanelType => Some(EntityUUID::PanelType(PanelTypeId::from_uuid(uuid))),
            EntityKind::PanelToPresenter => Some(EntityUUID::PanelToPresenter(
                PanelToPresenterId::from_uuid(uuid),
            )),
            EntityKind::PanelToEventRoom => Some(EntityUUID::PanelToEventRoom(
                PanelToEventRoomId::from_uuid(uuid),
            )),
            EntityKind::EventRoomToHotelRoom => Some(EntityUUID::EventRoomToHotelRoom(
                EventRoomToHotelRoomId::from_uuid(uuid),
            )),
            EntityKind::PanelToPanelType => Some(EntityUUID::PanelToPanelType(
                PanelToPanelTypeId::from_uuid(uuid),
            )),
            EntityKind::PresenterToGroup => Some(EntityUUID::PresenterToGroup(
                PresenterToGroupId::from_uuid(uuid),
            )),
        }
    }

    /// Register a UUID in the registry.
    fn register_uuid(&mut self, uuid: NonNilUuid, kind: EntityKind) -> Result<(), InsertError> {
        if let Some(&existing_kind) = self.uuid_registry.get(&uuid) {
            if existing_kind != kind {
                return Err(InsertError::UuidCollision { uuid });
            }
        } else {
            self.uuid_registry.insert(uuid, kind);
        }
        Ok(())
    }

    /// Unregister a UUID from the registry.
    fn unregister_uuid(&mut self, uuid: NonNilUuid) {
        self.uuid_registry.remove(&uuid);
    }

    // -----------------------------------------------------------------------
    // Generic entity CRUD (works for all node and edge entity types)
    // -----------------------------------------------------------------------

    /// Add any entity to the schedule, registering its UUID.
    ///
    /// For **node** entities (Panel, Presenter, …) this is the primary insertion
    /// method.  For **edge** entities prefer [`add_edge`](Self::add_edge) which
    /// additionally maintains the [`EdgeIndex`].
    pub fn add_entity<T>(&mut self, data: T::Data) -> Result<T::Id, InsertError>
    where
        T: EntityType + TypedStorage,
    {
        let uuid = data.uuid();
        self.register_uuid(uuid, T::KIND)?;
        EntityStore::<T>::insert_entity(&mut self.entities, uuid, data)?;
        Ok(T::Id::from_uuid(uuid))
    }

    /// Get entity data by typed ID.
    pub fn get_entity<T>(&self, id: T::Id) -> Option<&T::Data>
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::get_entity(&self.entities, id.non_nil_uuid())
    }

    /// Get a mutable reference to entity data by typed ID.
    pub fn get_entity_mut<T>(&mut self, id: T::Id) -> Option<&mut T::Data>
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::get_entity_mut(&mut self.entities, id.non_nil_uuid())
    }

    /// Remove an entity by typed ID, returning the data if it existed.
    ///
    /// For **edge** entities prefer [`remove_edge`](Self::remove_edge) which
    /// additionally cleans up the [`EdgeIndex`].
    pub fn remove_entity<T>(&mut self, id: T::Id) -> Option<T::Data>
    where
        T: EntityType + TypedStorage,
    {
        let uuid = id.non_nil_uuid();
        self.unregister_uuid(uuid);
        EntityStore::<T>::remove_entity(&mut self.entities, uuid)
    }

    /// Check if an entity with the given typed ID exists.
    pub fn contains_entity<T>(&self, id: T::Id) -> bool
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::contains_entity(&self.entities, id.non_nil_uuid())
    }

    /// Get entity data by raw UUID (requires knowing the entity type).
    pub fn get_entity_by_uuid<T>(&self, uuid: NonNilUuid) -> Option<&T::Data>
    where
        T: EntityType + TypedStorage,
    {
        EntityStore::<T>::get_entity(&self.entities, uuid)
    }

    // -----------------------------------------------------------------------
    // Edge entity CRUD (maintains EdgeIndex alongside entity storage)
    // -----------------------------------------------------------------------

    /// Add an edge entity and update the edge index.
    ///
    /// Registers the edge’s UUID, inserts the entity into storage, and records
    /// the from→to relationship in the [`EdgeIndex`] for the edge type.
    pub fn add_edge<T>(&mut self, data: T::Data) -> Result<T::Id, InsertError>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        let uuid = data.uuid();
        let from_uuid = data.from_uuid();
        let to_uuid = data.to_uuid();
        self.register_uuid(uuid, T::KIND)?;
        EntityStore::<T>::insert_entity(&mut self.entities, uuid, data)?;
        T::edge_index_mut(&mut self.entities).add(from_uuid, to_uuid, uuid);
        Ok(T::Id::from_uuid(uuid))
    }

    /// Remove an edge entity and update the edge index.
    ///
    /// Returns the edge data if it existed.  Both the UUID registry and the
    /// [`EdgeIndex`] are cleaned up.
    pub fn remove_edge<T>(&mut self, id: T::Id) -> Option<T::Data>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        let uuid = id.non_nil_uuid();
        let data = EntityStore::<T>::remove_entity(&mut self.entities, uuid)?;
        self.unregister_uuid(uuid);
        T::edge_index_mut(&mut self.entities).remove(data.from_uuid(), data.to_uuid(), uuid);
        Some(data)
    }

    // -----------------------------------------------------------------------
    // Edge queries
    // -----------------------------------------------------------------------

    /// Edge entity UUIDs leaving `from` for edge type `T`.
    pub fn edge_uuids_from<T>(&self, from: NonNilUuid) -> &[NonNilUuid]
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(&self.entities).outgoing(from)
    }

    /// Edge entity UUIDs arriving at `to` for edge type `T`.
    pub fn edge_uuids_to<T>(&self, to: NonNilUuid) -> &[NonNilUuid]
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(&self.entities).incoming(to)
    }

    /// Resolved edge data for all edges leaving `from`.
    pub fn edges_from<T>(&self, from: NonNilUuid) -> Vec<&T::Data>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(&self.entities)
            .outgoing(from)
            .iter()
            .filter_map(|&edge_uuid| self.entities.get::<T>(edge_uuid))
            .collect()
    }

    /// Resolved edge data for all edges arriving at `to`.
    pub fn edges_to<T>(&self, to: NonNilUuid) -> Vec<&T::Data>
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(&self.entities)
            .incoming(to)
            .iter()
            .filter_map(|&edge_uuid| self.entities.get::<T>(edge_uuid))
            .collect()
    }

    /// Check whether an edge of type `T` exists between `from` and `to`.
    pub fn edge_exists<T>(&self, from: NonNilUuid, to: NonNilUuid) -> bool
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(&self.entities)
            .outgoing(from)
            .iter()
            .any(|&edge_uuid| {
                self.entities
                    .get::<T>(edge_uuid)
                    .is_some_and(|data| data.to_uuid() == to)
            })
    }

    /// Number of edges of type `T` currently stored.
    pub fn edge_count<T>(&self) -> usize
    where
        T: TypedEdgeStorage,
        T::Data: DirectedEdge,
    {
        T::edge_index(&self.entities).len()
    }

    // -----------------------------------------------------------------------
    // Relationship convenience methods
    // -----------------------------------------------------------------------

    /// Presenters assigned to a panel.
    pub fn get_panel_presenters(&self, panel_id: PanelId) -> Vec<PresenterId> {
        PanelToPresenterEntityType::presenters_of(&self.entities, panel_id.non_nil_uuid())
    }

    /// Panels a presenter is assigned to.
    pub fn get_presenter_panels(&self, presenter_id: PresenterId) -> Vec<PanelId> {
        PanelToPresenterEntityType::panels_of(&self.entities, presenter_id.non_nil_uuid())
    }

    /// Event room assigned to a panel (at most one).
    pub fn get_panel_event_room(&self, panel_id: PanelId) -> Option<EventRoomId> {
        PanelToEventRoomEntityType::event_room_of(&self.entities, panel_id.non_nil_uuid())
    }

    /// Panels assigned to an event room.
    pub fn get_event_room_panels(&self, event_room_id: EventRoomId) -> Vec<PanelId> {
        PanelToEventRoomEntityType::panels_in(&self.entities, event_room_id.non_nil_uuid())
    }

    /// Panel type assigned to a panel (at most one).
    pub fn get_panel_type(&self, panel_id: PanelId) -> Option<PanelTypeId> {
        PanelToPanelTypeEntityType::panel_type_of(&self.entities, panel_id.non_nil_uuid())
    }

    /// Panels of a given panel type.
    pub fn get_panels_by_type(&self, panel_type_id: PanelTypeId) -> Vec<PanelId> {
        PanelToPanelTypeEntityType::panels_of_type(&self.entities, panel_type_id.non_nil_uuid())
    }

    /// Hotel rooms mapped to an event room.
    pub fn get_event_room_hotel_rooms(&self, event_room_id: EventRoomId) -> Vec<HotelRoomId> {
        EventRoomToHotelRoomEntityType::hotel_rooms_of(&self.entities, event_room_id.non_nil_uuid())
    }

    /// Groups a presenter belongs to (via outgoing PresenterToGroup edges).
    pub fn get_presenter_groups(&self, presenter_id: PresenterId) -> Vec<PresenterId> {
        PresenterToGroupEntityType::groups_of(&self.entities, presenter_id.non_nil_uuid())
    }

    /// Members of a presenter group (via incoming PresenterToGroup edges).
    pub fn get_presenter_members(&self, group_id: PresenterId) -> Vec<PresenterId> {
        PresenterToGroupEntityType::members_of(&self.entities, group_id.non_nil_uuid())
    }

    // -----------------------------------------------------------------------
    // Generic name lookup helper used by computed-field closures
    // -----------------------------------------------------------------------

    /// Return display names for a slice of UUIDs belonging to entity type `T`.
    /// TODO: Implement field-based name lookup when field system is fully integrated.
    pub fn get_entity_names<T: EntityType>(&self, _uuids: &[NonNilUuid]) -> Vec<String> {
        vec![]
    }
}

/// `Schedule` delegates `EntityStore<T>` to its inner `EntityStorage`,
/// adding UUID registry management.
impl<T: TypedStorage> EntityStore<T> for Schedule {
    fn get_entity(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        EntityStore::<T>::get_entity(&self.entities, uuid)
    }

    fn get_entity_mut(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data> {
        EntityStore::<T>::get_entity_mut(&mut self.entities, uuid)
    }

    fn insert_entity(&mut self, uuid: NonNilUuid, data: T::Data) -> Result<(), InsertError> {
        self.register_uuid(uuid, T::KIND)?;
        EntityStore::<T>::insert_entity(&mut self.entities, uuid, data)
    }

    fn remove_entity(&mut self, uuid: NonNilUuid) -> Option<T::Data> {
        self.unregister_uuid(uuid);
        EntityStore::<T>::remove_entity(&mut self.entities, uuid)
    }

    fn contains_entity(&self, uuid: NonNilUuid) -> bool {
        EntityStore::<T>::contains_entity(&self.entities, uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{
        EventRoomToHotelRoomData, PanelToEventRoomData, PanelToPanelTypeData, PanelToPresenterData,
        PresenterToGroupData, UuidPreference,
    };
    use uuid::Uuid;

    fn nn(b: u8) -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b,
            ]))
        }
    }

    /// Insert a Panel with the given UUID, uid, and name. Returns PanelId.
    fn add_panel(schedule: &mut Schedule, uuid: NonNilUuid, uid: &str, name: &str) -> PanelId {
        crate::entity::panel::PanelBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_uid(uid.to_string())
            .with_name(name.to_string())
            .build(schedule)
            .unwrap()
    }

    /// Insert a Presenter with the given UUID and name. Returns PresenterId.
    fn add_presenter(schedule: &mut Schedule, uuid: NonNilUuid, name: &str) -> PresenterId {
        crate::entity::presenter::PresenterBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_name(name.to_string())
            .build(schedule)
            .unwrap()
    }

    /// Insert an EventRoom with the given UUID and name. Returns EventRoomId.
    fn add_event_room(schedule: &mut Schedule, uuid: NonNilUuid, name: &str) -> EventRoomId {
        crate::entity::event_room::EventRoomBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_room_name(name.to_string())
            .build(schedule)
            .unwrap()
    }

    /// Insert a PanelType with the given UUID, prefix, and kind. Returns PanelTypeId.
    fn add_panel_type(
        schedule: &mut Schedule,
        uuid: NonNilUuid,
        prefix: &str,
        kind: &str,
    ) -> PanelTypeId {
        crate::entity::panel_type::PanelTypeBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_prefix(prefix.to_string())
            .with_panel_kind(kind.to_string())
            .build(schedule)
            .unwrap()
    }

    /// Insert a HotelRoom with the given UUID and name. Returns HotelRoomId.
    fn add_hotel_room(schedule: &mut Schedule, uuid: NonNilUuid, name: &str) -> HotelRoomId {
        crate::entity::hotel_room::HotelRoomBuilder::new()
            .with_uuid_preference(UuidPreference::Exact(uuid))
            .with_hotel_room_name(name.to_string())
            .build(schedule)
            .unwrap()
    }

    #[test]
    fn test_add_edge_panel_to_presenter() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        let presenter_uuid = nn(2);
        let edge_uuid = nn(10);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        add_presenter(&mut schedule, presenter_uuid, "Alice");

        let edge_id = schedule
            .add_edge::<PanelToPresenterEntityType>(PanelToPresenterData {
                entity_uuid: edge_uuid,
                panel_uuid,
                presenter_uuid,
            })
            .unwrap();
        assert_eq!(edge_id.non_nil_uuid(), edge_uuid);

        // Query via convenience method
        let panel_id = PanelId::from(panel_uuid);
        let presenters = schedule.get_panel_presenters(panel_id);
        assert_eq!(presenters.len(), 1);
        assert_eq!(presenters[0].non_nil_uuid(), presenter_uuid);

        // Reverse query
        let presenter_id = PresenterId::from(presenter_uuid);
        let panels = schedule.get_presenter_panels(presenter_id);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].non_nil_uuid(), panel_uuid);
    }

    #[test]
    fn test_remove_edge_updates_index() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        let presenter_uuid = nn(2);
        let edge_uuid = nn(10);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        add_presenter(&mut schedule, presenter_uuid, "Bob");

        let edge_id = schedule
            .add_edge::<PanelToPresenterEntityType>(PanelToPresenterData {
                entity_uuid: edge_uuid,
                panel_uuid,
                presenter_uuid,
            })
            .unwrap();

        let removed = schedule.remove_edge::<PanelToPresenterEntityType>(edge_id);
        assert!(removed.is_some());

        let panel_id = PanelId::from(panel_uuid);
        assert!(schedule.get_panel_presenters(panel_id).is_empty());
        assert_eq!(schedule.edge_count::<PanelToPresenterEntityType>(), 0);
    }

    #[test]
    fn test_multiple_presenters_per_panel() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        let p1 = nn(2);
        let p2 = nn(3);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        add_presenter(&mut schedule, p1, "Alice");
        add_presenter(&mut schedule, p2, "Bob");

        schedule
            .add_edge::<PanelToPresenterEntityType>(PanelToPresenterData {
                entity_uuid: nn(10),
                panel_uuid,
                presenter_uuid: p1,
            })
            .unwrap();
        schedule
            .add_edge::<PanelToPresenterEntityType>(PanelToPresenterData {
                entity_uuid: nn(11),
                panel_uuid,
                presenter_uuid: p2,
            })
            .unwrap();

        let panel_id = PanelId::from(panel_uuid);
        let presenters = schedule.get_panel_presenters(panel_id);
        assert_eq!(presenters.len(), 2);
        assert_eq!(schedule.edge_count::<PanelToPresenterEntityType>(), 2);
    }

    #[test]
    fn test_panel_to_event_room_single() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        let room_uuid = nn(2);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        add_event_room(&mut schedule, room_uuid, "Room A");

        schedule
            .add_edge::<PanelToEventRoomEntityType>(PanelToEventRoomData {
                entity_uuid: nn(10),
                panel_uuid,
                event_room_uuid: room_uuid,
            })
            .unwrap();

        let panel_id = PanelId::from(panel_uuid);
        let room = schedule.get_panel_event_room(panel_id);
        assert!(room.is_some());
        assert_eq!(room.unwrap().non_nil_uuid(), room_uuid);
    }

    #[test]
    fn test_panel_to_panel_type() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        let type_uuid = nn(2);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        add_panel_type(&mut schedule, type_uuid, "WS", "Workshop");

        schedule
            .add_edge::<PanelToPanelTypeEntityType>(PanelToPanelTypeData {
                entity_uuid: nn(10),
                panel_uuid,
                panel_type_uuid: type_uuid,
            })
            .unwrap();

        let panel_id = PanelId::from(panel_uuid);
        assert_eq!(
            schedule.get_panel_type(panel_id).unwrap().non_nil_uuid(),
            type_uuid
        );

        let type_id = PanelTypeId::from(type_uuid);
        let panels = schedule.get_panels_by_type(type_id);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].non_nil_uuid(), panel_uuid);
    }

    #[test]
    fn test_event_room_to_hotel_room() {
        let mut schedule = Schedule::new();
        let er_uuid = nn(1);
        let hr_uuid = nn(2);

        add_event_room(&mut schedule, er_uuid, "ER1");
        add_hotel_room(&mut schedule, hr_uuid, "HR1");

        schedule
            .add_edge::<EventRoomToHotelRoomEntityType>(EventRoomToHotelRoomData {
                entity_uuid: nn(10),
                event_room_uuid: er_uuid,
                hotel_room_uuid: hr_uuid,
            })
            .unwrap();

        let er_id = EventRoomId::from(er_uuid);
        let rooms = schedule.get_event_room_hotel_rooms(er_id);
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].non_nil_uuid(), hr_uuid);
    }

    #[test]
    fn test_presenter_to_group() {
        let mut schedule = Schedule::new();
        let member_uuid = nn(1);
        let group_uuid = nn(2);

        add_presenter(&mut schedule, member_uuid, "Alice");
        add_presenter(&mut schedule, group_uuid, "Group A");

        // Group marker (self-loop)
        schedule
            .add_edge::<PresenterToGroupEntityType>(PresenterToGroupData {
                entity_uuid: nn(10),
                member_uuid: group_uuid,
                group_uuid,
                always_shown_in_group: false,
                always_grouped: false,
            })
            .unwrap();

        // Membership edge
        schedule
            .add_edge::<PresenterToGroupEntityType>(PresenterToGroupData {
                entity_uuid: nn(11),
                member_uuid,
                group_uuid,
                always_shown_in_group: true,
                always_grouped: false,
            })
            .unwrap();

        let member_id = PresenterId::from(member_uuid);
        let groups = schedule.get_presenter_groups(member_id);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].non_nil_uuid(), group_uuid);

        let group_id = PresenterId::from(group_uuid);
        let members = schedule.get_presenter_members(group_id);
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].non_nil_uuid(), member_uuid);

        // is_group check via entity type
        assert!(PresenterToGroupEntityType::is_group(
            &schedule.entities,
            group_uuid
        ));
        assert!(!PresenterToGroupEntityType::is_group(
            &schedule.entities,
            member_uuid
        ));
    }

    #[test]
    fn test_edge_exists() {
        let mut schedule = Schedule::new();
        let panel_uuid = nn(1);
        let presenter_uuid = nn(2);

        add_panel(&mut schedule, panel_uuid, "P1", "Panel 1");
        add_presenter(&mut schedule, presenter_uuid, "Alice");

        assert!(!schedule.edge_exists::<PanelToPresenterEntityType>(panel_uuid, presenter_uuid));

        schedule
            .add_edge::<PanelToPresenterEntityType>(PanelToPresenterData {
                entity_uuid: nn(10),
                panel_uuid,
                presenter_uuid,
            })
            .unwrap();

        assert!(schedule.edge_exists::<PanelToPresenterEntityType>(panel_uuid, presenter_uuid));
    }

    #[test]
    fn test_edge_uuid_collision_rejected() {
        let mut schedule = Schedule::new();
        let edge = PanelToPresenterData {
            entity_uuid: nn(10),
            panel_uuid: nn(1),
            presenter_uuid: nn(2),
        };
        schedule
            .add_edge::<PanelToPresenterEntityType>(edge.clone())
            .unwrap();

        let edge2 = PanelToPresenterData {
            entity_uuid: nn(10), // same UUID
            panel_uuid: nn(3),
            presenter_uuid: nn(4),
        };
        let result = schedule.add_edge::<PanelToPresenterEntityType>(edge2);
        assert!(result.is_err());
    }

    #[test]
    fn test_identify_edge() {
        let mut schedule = Schedule::new();
        let edge_uuid = nn(10);
        schedule
            .add_edge::<PanelToPresenterEntityType>(PanelToPresenterData {
                entity_uuid: edge_uuid,
                panel_uuid: nn(1),
                presenter_uuid: nn(2),
            })
            .unwrap();

        let identified = schedule.identify(edge_uuid);
        assert!(matches!(identified, Some(EntityUUID::PanelToPresenter(_))));
    }
}
