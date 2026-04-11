/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule container with entity storage, edge indexing, and UUID registry.

mod metadata;
mod storage;

pub use metadata::{GeneratorInfo, ScheduleMetadata};
pub use storage::{BuildError, EntityStorage, EntityStore, InsertError, TypedStorage};

use crate::entity::{
    EntityKind, EntityType, EntityUUID, EventRoomId, EventRoomToHotelRoomId, HotelRoomId,
    InternalData, PanelId, PanelToEventRoomId, PanelToPanelTypeId, PanelToPresenterId, PanelTypeId,
    PresenterId, PresenterToGroupId, TypedId,
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
    /// Works for both node entities (Panel, Presenter, …) and edge entities
    /// (PanelToPresenter, …).  The correct `HashMap` is selected at compile
    /// time via [`TypedStorage`].
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
    // Relationship convenience methods
    // -----------------------------------------------------------------------

    /// Get all groups a presenter belongs to.
    /// TODO: Implement using presenter_to_group edge storage
    pub fn get_presenter_groups(&self, _presenter_id: PresenterId) -> Vec<PresenterId> {
        vec![]
    }

    /// Get all members of a presenter group.
    /// TODO: Implement using presenter_to_group edge storage
    pub fn get_presenter_members(&self, _presenter_id: PresenterId) -> Vec<PresenterId> {
        vec![]
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

    // #[test]
    // fn test_schedule_new() {
    //     let schedule = Schedule::new();
    //     assert_eq!(schedule.panels.len(), 0);
    //     assert_eq!(schedule.presenters.len(), 0);
    //     assert!(schedule.uuid_registry.is_empty());
    // }

    // #[test]
    // fn test_schedule_add_panel() {
    //     let mut schedule = Schedule::new();
    //     let panel = PanelBuilder::new()
    //         .with_name("Test Panel")
    //         .with_uid("GW001")
    //         .build()
    //         .unwrap();
    //     let data = panel.into_data();
    //     let uuid = data.uuid();

    //     let panel_id = schedule.add_panel(data).unwrap();
    //     assert_eq!(panel_id.non_nil_uuid(), uuid);
    //     assert!(schedule.panels.contains(uuid));
    //     assert!(schedule.uuid_registry.contains_key(&uuid));
    // }

    // #[test]
    // fn test_schedule_uuid_collision() {
    //     let mut schedule = Schedule::new();
    //     let panel = PanelBuilder::new()
    //         .with_name("Test Panel")
    //         .with_uid("GW001")
    //         .build()
    //         .unwrap();
    //     let data = panel.into_data();
    //     let uuid = data.uuid();

    //     schedule.add_panel(data.clone()).unwrap();
    //     let result = schedule.add_panel(data);
    //     assert!(matches!(result, Err(InsertError::UuidCollision { .. })));
    // }

    // #[test]
    // fn test_schedule_connect_panel_to_presenter() {
    //     let mut schedule = Schedule::new();

    //     let panel = PanelBuilder::new()
    //         .with_name("Test Panel")
    //         .with_uid("GW001")
    //         .build()
    //         .unwrap();
    //     let panel_data = panel.into_data();
    //     let panel_id = schedule.add_panel(panel_data).unwrap();

    //     let presenter = PresenterBuilder::new()
    //         .with_name("Test Presenter")
    //         .build()
    //         .unwrap();
    //     let presenter_data = presenter.into_data();
    //     let presenter_id = schedule.add_presenter(presenter_data).unwrap();

    //     let edge = crate::entity::panel_to_presenter::PanelToPresenterBuilder::new()
    //         .with_from_id(panel_id)
    //         .with_to_id(presenter_id)
    //         .build()
    //         .unwrap();
    //     let edge_data = edge.into_data();

    //     let edge_id = schedule.connect_panel_to_presenter(edge_data).unwrap();
    //     assert!(schedule
    //         .panel_to_presenters
    //         .contains(edge_id.non_nil_uuid()));

    //     let presenters = schedule.get_panel_presenters(panel_id);
    //     assert_eq!(presenters, vec![presenter_id]);
    // }

    // #[test]
    // fn test_schedule_identify() {
    //     let mut schedule = Schedule::new();
    //     let panel = PanelBuilder::new()
    //         .with_name("Test Panel")
    //         .with_uid("GW001")
    //         .build()
    //         .unwrap();
    //     let data = panel.into_data();
    //     let uuid = data.uuid();

    //     schedule.add_panel(data).unwrap();
    //     let identified = schedule.identify(uuid);
    //     assert!(matches!(identified, Some(EntityUUID::Panel(_))));
    // }
}
