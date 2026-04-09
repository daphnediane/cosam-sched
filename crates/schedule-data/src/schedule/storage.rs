/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity and edge storage implementation

use std::collections::HashMap;

use super::ScheduleError;
use crate::entity::{
    EntityType, EventRoomData, EventRoomEntityType, EventRoomToHotelRoomData,
    EventRoomToHotelRoomEntityType, HotelRoomData, HotelRoomEntityType, PanelData, PanelEntityType,
    PanelToEventRoomData, PanelToEventRoomEntityType, PanelToPanelTypeData,
    PanelToPanelTypeEntityType, PanelToPresenterData, PanelToPresenterEntityType, PanelTypeData,
    PanelTypeEntityType, PresenterData, PresenterEntityType, PresenterToGroupData,
    PresenterToGroupEntityType,
};
use crate::field::FieldValue;
use crate::query::{FieldMatch, QueryOptions};
use uuid::NonNilUuid;

/// Concrete typed entity storage — one `HashMap` per entity type.
/// This avoids type erasure and allows direct `&T::Data` references.
#[derive(Debug, Clone, Default)]
pub struct EntityStorage {
    pub panels: HashMap<NonNilUuid, PanelData>,
    pub presenters: HashMap<NonNilUuid, PresenterData>,
    pub event_rooms: HashMap<NonNilUuid, EventRoomData>,
    pub hotel_rooms: HashMap<NonNilUuid, HotelRoomData>,
    pub panel_types: HashMap<NonNilUuid, PanelTypeData>,
    pub panel_to_presenters: HashMap<NonNilUuid, PanelToPresenterData>,
    pub panel_to_event_rooms: HashMap<NonNilUuid, PanelToEventRoomData>,
    pub event_room_to_hotel_rooms: HashMap<NonNilUuid, EventRoomToHotelRoomData>,
    pub panel_to_panel_types: HashMap<NonNilUuid, PanelToPanelTypeData>,
    pub presenter_to_groups: HashMap<NonNilUuid, PresenterToGroupData>,
}

/// Provides access to the concrete `HashMap` for an entity type.
/// Implemented on `EntityType` marker structs.
pub trait TypedStorage: EntityType {
    fn typed_map(storage: &EntityStorage) -> &HashMap<NonNilUuid, Self::Data>;
    fn typed_map_mut(storage: &mut EntityStorage) -> &mut HashMap<NonNilUuid, Self::Data>;
}

impl TypedStorage for PanelEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelData> {
        &s.panels
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelData> {
        &mut s.panels
    }
}

impl TypedStorage for PresenterEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PresenterData> {
        &s.presenters
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PresenterData> {
        &mut s.presenters
    }
}

impl TypedStorage for EventRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, EventRoomData> {
        &s.event_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, EventRoomData> {
        &mut s.event_rooms
    }
}

impl TypedStorage for HotelRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, HotelRoomData> {
        &s.hotel_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, HotelRoomData> {
        &mut s.hotel_rooms
    }
}

impl TypedStorage for PanelTypeEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelTypeData> {
        &s.panel_types
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelTypeData> {
        &mut s.panel_types
    }
}

impl TypedStorage for PanelToPresenterEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelToPresenterData> {
        &s.panel_to_presenters
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelToPresenterData> {
        &mut s.panel_to_presenters
    }
}

impl TypedStorage for PanelToEventRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelToEventRoomData> {
        &s.panel_to_event_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelToEventRoomData> {
        &mut s.panel_to_event_rooms
    }
}

impl TypedStorage for EventRoomToHotelRoomEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, EventRoomToHotelRoomData> {
        &s.event_room_to_hotel_rooms
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, EventRoomToHotelRoomData> {
        &mut s.event_room_to_hotel_rooms
    }
}

impl TypedStorage for PanelToPanelTypeEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PanelToPanelTypeData> {
        &s.panel_to_panel_types
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PanelToPanelTypeData> {
        &mut s.panel_to_panel_types
    }
}

impl TypedStorage for PresenterToGroupEntityType {
    fn typed_map(s: &EntityStorage) -> &HashMap<NonNilUuid, PresenterToGroupData> {
        &s.presenter_to_groups
    }
    fn typed_map_mut(s: &mut EntityStorage) -> &mut HashMap<NonNilUuid, PresenterToGroupData> {
        &mut s.presenter_to_groups
    }
}

impl EntityStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get entity by type and UUID.
    pub fn get<T: TypedStorage>(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        T::typed_map(self).get(&uuid)
    }

    /// Get entity by internal UUID (alias for `get`).
    pub fn get_by_uuid<T: TypedStorage>(&self, uuid: NonNilUuid) -> Option<&T::Data> {
        T::typed_map(self).get(&uuid)
    }

    /// Get entities by index query, returning all that tie at the best match strength.
    pub fn get_by_index<T: TypedStorage + Sized>(&self, query: &str) -> Vec<&T::Data> {
        let field_set = T::field_set();
        let map = T::typed_map(self);

        let mut best_priority = crate::field::traits::match_priority::MIN_MATCH;
        let mut matched_uuids: Vec<NonNilUuid> = Vec::new();

        // @TODO: Should consider field priority if match priority is the same
        for entity in map.values() {
            if let Some(match_result) = field_set.match_index(query, entity) {
                if match_result.priority > best_priority {
                    best_priority = match_result.priority;
                    matched_uuids.clear();
                    matched_uuids.push(match_result.entity_uuid);
                } else if match_result.priority == best_priority {
                    matched_uuids.push(match_result.entity_uuid);
                }
            }
        }

        matched_uuids
            .into_iter()
            .filter_map(|id| map.get(&id))
            .collect()
    }

    /// Get multiple entities matching field conditions.
    pub fn get_many<T: TypedStorage + Sized>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<&T::Data> {
        let ids = self.find::<T>(matches, options);
        ids.into_iter()
            .filter_map(|id| T::typed_map(self).get(&id))
            .collect()
    }

    /// Find entity UUIDs matching field conditions.
    pub fn find<T: TypedStorage + Sized>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<NonNilUuid> {
        let options = options.unwrap_or_default();
        let map = T::typed_map(self);

        let mut results: Vec<NonNilUuid> = map
            .iter()
            .filter(|(_uuid, _entity)| {
                for field_match in matches {
                    if T::field_set().get_field(&field_match.field_name).is_none() {
                        return false;
                    }
                    // TODO: Implement proper field value matching
                }
                true
            })
            .map(|(uuid, _)| *uuid)
            .collect();

        // Apply ordering
        if let Some(_order_by) = options.order_by {
            results.sort_by(|a, b| {
                if options.ascending {
                    a.to_string().cmp(&b.to_string())
                } else {
                    b.to_string().cmp(&a.to_string())
                }
            });
        }

        // Apply limit and offset
        let start = options.offset.unwrap_or(0);
        let end = options
            .limit
            .map(|l| (start + l).min(results.len()))
            .unwrap_or(results.len());

        results.into_iter().skip(start).take(end - start).collect()
    }

    /// Add entity to storage with pre-allocated UUID.
    pub fn add_with_uuid<T: TypedStorage>(
        &mut self,
        uuid: NonNilUuid,
        entity: T::Data,
    ) -> Result<(), ScheduleError> {
        let map = T::typed_map_mut(self);
        if map.contains_key(&uuid) {
            return Err(ScheduleError::DuplicateEntity {
                entity_type: T::TYPE_NAME.to_string(),
                id: uuid.to_string(),
            });
        }
        map.insert(uuid, entity);
        Ok(())
    }

    /// Check if entity with given UUID exists.
    pub fn contains_uuid<T: TypedStorage>(&self, uuid: NonNilUuid) -> bool {
        T::typed_map(self).contains_key(&uuid)
    }

    /// Update entity fields.
    /// TODO: Apply individual field updates once field system supports mutation.
    pub fn update<T: TypedStorage>(
        &mut self,
        uuid: NonNilUuid,
        _updates: &[(String, FieldValue)],
    ) -> Result<(), ScheduleError> {
        if T::typed_map(self).contains_key(&uuid) {
            Ok(())
        } else {
            Err(ScheduleError::EntityNotFound {
                entity_type: T::TYPE_NAME.to_string(),
                id: uuid.to_string(),
            })
        }
    }
}
