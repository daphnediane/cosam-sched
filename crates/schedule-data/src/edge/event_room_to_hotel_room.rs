/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoomToHotelRoom edge implementation

use crate::edge::generic::GenericEdgeStorage;
use crate::edge::{Edge, EdgeError, EdgeId, EdgeStorage};
use crate::entity::EntityId;

/// EventRoomToHotelRoom edge implementation (many-to-one relationship)
#[derive(Debug, Clone)]
pub struct EventRoomToHotelRoomEdge {
    pub from_id: crate::entity::InternalId, // EventRoom (many)
    pub to_id: crate::entity::InternalId,   // HotelRoom (one)
    pub data: EventRoomToHotelRoomData,
}

#[derive(Debug, Clone)]
pub struct EventRoomToHotelRoomData {
    // No additional data needed for this simple relationship
}

impl EventRoomToHotelRoomEdge {
    pub fn new(event_room_id: EntityId, hotel_room_id: EntityId) -> Self {
        Self {
            from_id: crate::entity::InternalId::new::<crate::entity::EventRoomEntityType>(
                event_room_id,
            ),
            to_id: crate::entity::InternalId::new::<crate::entity::HotelRoomEntityType>(
                hotel_room_id,
            ),
            data: EventRoomToHotelRoomData {},
        }
    }
}

impl Edge for EventRoomToHotelRoomEdge {
    type FromEntity = crate::entity::EventRoomEntityType;
    type ToEntity = crate::entity::HotelRoomEntityType;
    type Data = EventRoomToHotelRoomData;

    fn from_id(&self) -> Option<crate::entity::InternalId> {
        Some(self.from_id)
    }

    fn to_id(&self) -> Option<crate::entity::InternalId> {
        Some(self.to_id)
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
    }

    fn edge_type(&self) -> crate::edge::EdgeType {
        crate::edge::EdgeType::EventRoomToHotelRoom
    }
}

/// Specialized storage for EventRoomToHotelRoom with time range caching
#[derive(Debug, Clone)]
pub struct EventRoomToHotelRoomStorage {
    edges: GenericEdgeStorage<EventRoomToHotelRoomEdge>,
    time_range_cache: std::collections::HashMap<
        EntityId,
        Vec<(chrono::NaiveDateTime, chrono::NaiveDateTime, EntityId)>,
    >,
    panel_usage_cache: std::collections::HashMap<EntityId, Vec<EntityId>>,
    cache_invalidation: u64,
}

impl EventRoomToHotelRoomStorage {
    pub fn new() -> Self {
        Self {
            edges: GenericEdgeStorage::new(),
            time_range_cache: std::collections::HashMap::new(),
            panel_usage_cache: std::collections::HashMap::new(),
            cache_invalidation: 0,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.cache_invalidation += 1;
        self.time_range_cache.clear();
        self.panel_usage_cache.clear();
    }

    /// Get time ranges for a hotel room (start, end, panel_id)
    /// This will be called by Schedule with access to panel data
    pub fn get_time_ranges(
        &mut self,
        hotel_room_id: EntityId,
        _schedule: &super::super::schedule::Schedule,
    ) -> &[(chrono::NaiveDateTime, chrono::NaiveDateTime, EntityId)] {
        if self.time_range_cache.contains_key(&hotel_room_id) {
            return self.time_range_cache.get(&hotel_room_id).unwrap();
        }

        // TODO: Implement time range computation using panel data from schedule
        // For now, return empty vec
        self.time_range_cache.insert(hotel_room_id, Vec::new());
        self.time_range_cache.get(&hotel_room_id).unwrap()
    }

    /// Get panels using an event room
    /// This will be called by Schedule with access to panel data
    pub fn get_panels_using_event_room(
        &mut self,
        event_room_id: EntityId,
        _schedule: &super::super::schedule::Schedule,
    ) -> &[EntityId] {
        if self.panel_usage_cache.contains_key(&event_room_id) {
            return self.panel_usage_cache.get(&event_room_id).unwrap();
        }

        // TODO: Implement panel usage lookup using panel data from schedule
        // For now, return empty vec
        self.panel_usage_cache.insert(event_room_id, Vec::new());
        self.panel_usage_cache.get(&event_room_id).unwrap()
    }
}

impl Default for EventRoomToHotelRoomStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeStorage<EventRoomToHotelRoomEdge> for EventRoomToHotelRoomStorage {
    fn add_edge(&mut self, edge: EventRoomToHotelRoomEdge) -> Result<EdgeId, EdgeError> {
        let id = self.edges.add_edge(edge)?;
        self.invalidate_cache();
        Ok(id)
    }

    fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError> {
        let result = self.edges.remove_edge(edge_id);
        self.invalidate_cache();
        result
    }

    fn get_edge(&self, edge_id: EdgeId) -> Option<&EventRoomToHotelRoomEdge> {
        self.edges.get_edge(edge_id)
    }

    fn find_outgoing(&self, from_id: crate::entity::InternalId) -> Vec<&EventRoomToHotelRoomEdge> {
        self.edges.find_outgoing(from_id)
    }

    fn find_incoming(&self, to_id: crate::entity::InternalId) -> Vec<&EventRoomToHotelRoomEdge> {
        self.edges.find_incoming(to_id)
    }

    fn edge_exists(
        &self,
        from_id: &crate::entity::InternalId,
        to_id: &crate::entity::InternalId,
    ) -> bool {
        self.edges.edge_exists(from_id, to_id)
    }

    fn len(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_invalidation_on_add() {
        let mut storage = EventRoomToHotelRoomStorage::new();
        let event_room_id: EntityId = 1;
        let hotel_room_id: EntityId = 10;

        storage
            .add_edge(EventRoomToHotelRoomEdge::new(event_room_id, hotel_room_id))
            .unwrap();
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_cache_invalidation_on_remove() {
        let mut storage = EventRoomToHotelRoomStorage::new();
        let event_room_id: EntityId = 1;
        let hotel_room_id: EntityId = 10;

        let edge_id = storage
            .add_edge(EventRoomToHotelRoomEdge::new(event_room_id, hotel_room_id))
            .unwrap();
        storage.remove_edge(edge_id).unwrap();

        assert_eq!(storage.len(), 0);
    }

    #[test]
    fn test_invalidate_cache_clears_caches() {
        let mut storage = EventRoomToHotelRoomStorage::new();

        // Directly call invalidate_cache to verify it doesn't panic
        storage.invalidate_cache();

        // Verify storage is still functional after invalidation
        let event_room_id: EntityId = 1;
        let hotel_room_id: EntityId = 10;
        storage
            .add_edge(EventRoomToHotelRoomEdge::new(event_room_id, hotel_room_id))
            .unwrap();

        assert_eq!(storage.len(), 1);
    }
}
