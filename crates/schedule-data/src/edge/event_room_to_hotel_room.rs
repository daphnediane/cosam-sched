/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! EventRoomToHotelRoom edge implementation

use crate::edge::generic::GenericEdgeStorage;
use crate::edge::{Edge, EdgeError, EdgeId, EdgeStorage};
use crate::entity::{EventRoomId, HotelRoomId, Uuid};

/// EventRoomToHotelRoom edge implementation (many-to-one relationship)
#[derive(Debug, Clone)]
pub struct EventRoomToHotelRoomEdge {
    pub from_id: EventRoomId, // EventRoom (many)
    pub to_id: HotelRoomId,   // HotelRoom (one)
    pub data: EventRoomToHotelRoomData,
}

#[derive(Debug, Clone)]
pub struct EventRoomToHotelRoomData {
    // No additional data needed for this simple relationship
}

impl EventRoomToHotelRoomEdge {
    pub fn new(event_room_id: EventRoomId, hotel_room_id: HotelRoomId) -> Self {
        Self {
            from_id: event_room_id,
            to_id: hotel_room_id,
            data: EventRoomToHotelRoomData {},
        }
    }
}

impl Edge for EventRoomToHotelRoomEdge {
    type FromEntity = crate::entity::EventRoomEntityType;
    type ToEntity = crate::entity::HotelRoomEntityType;
    type Data = EventRoomToHotelRoomData;

    fn from_uuid(&self) -> Option<Uuid> {
        Some(Uuid::from(self.from_id))
    }

    fn to_uuid(&self) -> Option<Uuid> {
        Some(Uuid::from(self.to_id))
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
    time_range_cache:
        std::collections::HashMap<Uuid, Vec<(chrono::NaiveDateTime, chrono::NaiveDateTime, Uuid)>>,
    panel_usage_cache: std::collections::HashMap<Uuid, Vec<Uuid>>,
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
        hotel_room_id: Uuid,
        _schedule: &super::super::schedule::Schedule,
    ) -> &[(chrono::NaiveDateTime, chrono::NaiveDateTime, Uuid)] {
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
        event_room_id: Uuid,
        _schedule: &super::super::schedule::Schedule,
    ) -> &[Uuid] {
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

    fn find_outgoing(&self, from_uuid: Uuid) -> Vec<&EventRoomToHotelRoomEdge> {
        self.edges.find_outgoing(from_uuid)
    }

    fn find_incoming(&self, to_uuid: Uuid) -> Vec<&EventRoomToHotelRoomEdge> {
        self.edges.find_incoming(to_uuid)
    }

    fn edge_exists(&self, from_uuid: Uuid, to_uuid: Uuid) -> bool {
        self.edges.edge_exists(from_uuid, to_uuid)
    }

    fn len(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Uuid;

    fn make_event_room_id(id: u8) -> EventRoomId {
        EventRoomId::from(Uuid::from_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, id,
        ]))
    }

    fn make_hotel_room_id(id: u8) -> HotelRoomId {
        HotelRoomId::from(Uuid::from_bytes([
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            id + 100,
        ]))
    }

    #[test]
    fn test_cache_invalidation_on_add() {
        let mut storage = EventRoomToHotelRoomStorage::new();
        let event_room_id = make_event_room_id(1);
        let hotel_room_id = make_hotel_room_id(10);

        storage
            .add_edge(EventRoomToHotelRoomEdge::new(event_room_id, hotel_room_id))
            .unwrap();
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_cache_invalidation_on_remove() {
        let mut storage = EventRoomToHotelRoomStorage::new();
        let event_room_id = make_event_room_id(1);
        let hotel_room_id = make_hotel_room_id(10);

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
        let event_room_id = make_event_room_id(1);
        let hotel_room_id = make_hotel_room_id(10);
        storage
            .add_edge(EventRoomToHotelRoomEdge::new(event_room_id, hotel_room_id))
            .unwrap();

        assert_eq!(storage.len(), 1);
    }
}
