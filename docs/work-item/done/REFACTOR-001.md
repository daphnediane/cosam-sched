# Edge System Integration and Specialized Storage

## Summary

Replace Schedule's string-based EdgeStorage with type-safe edge storage system using typed edge structs and dedicated storage per relationship type. Foundation is complete; remaining work is integration into Schedule and implementing relationship-specific behaviors.

## Status

Completed

## Priority

High

## Description

The edge trait system and typed edge structs are implemented, but Schedule still uses the old generic string-based EdgeStorage. Need to integrate the new type-safe edge storage system and implement relationship-specific behaviors like transitive closures, time range caching, and cardinality constraints.

## Completed Work

- Edge trait hierarchy: `Edge`, `RelationshipEdge`, `SimpleEdge` in `edge/traits.rs`
- Generic `EdgeStorage<E: Edge>` struct with bidirectional index lookups in `edge/storage.rs` (to be converted to trait)
- All specific edge structs implemented:
  - `PresenterToGroupEdge` in `edge/presenter_to_group.rs`
  - `PanelToPresenterEdge` in `edge/panel_to_presenter.rs`
  - `PanelToEventRoomEdge` in `edge/panel_to_event_room.rs`
  - `EventRoomToHotelRoomEdge` in `edge/event_room_to_hotel_room.rs`
  - `PanelToPanelTypeEdge` in `edge/panel_to_panel_type.rs`
- `PresenterMemberToGroupStorage` with transitive closure matching schedule-core's `RelationshipManager`
- Old generic `Edge` entity removed from `entity/edge.rs`
- `RelationshipStorage` trait for relationship operations

## Remaining Work

### 1. Convert EdgeStorage to Trait-Based Design

Convert the current generic `EdgeStorage<E: Edge>` struct to a trait that specialized storage implementations implement:

**Trait definition:**

```rust
pub trait EdgeStorage<E: Edge> {
    fn add_edge(&mut self, edge: E) -> Result<EdgeId, EdgeError>;
    fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError>;
    fn get_edge(&self, edge_id: EdgeId) -> Option<&E>;
    fn find_outgoing(&self, from_id: EntityId) -> Vec<&E>;
    fn find_incoming(&self, to_id: EntityId) -> Vec<&E>;
    fn edge_exists(&self, from_id: &EntityId, to_id: &EntityId) -> bool;
    fn len(&self) -> usize;
}
```

**Implementations:**

- Create `GenericEdgeStorage<E: Edge>` struct implementing `EdgeStorage<E>` for simple edge types
- Update `PresenterMemberToGroupStorage` to implement `EdgeStorage<PresenterMemberToGroupEdge>`
- Future specialized storages will implement their respective `EdgeStorage<EdgeType>`

### 2. Integrate Type-Safe Edge Storage into Schedule

Replace `Schedule.edges: EdgeStorage` (string-based) with typed edge storage instances:

```rust
pub struct Schedule {
    pub entities: EntityStorage,
    // Replace generic EdgeStorage with typed storage per relationship
    pub presenter_to_group: PresenterMemberToGroupStorage,
    pub panel_to_presenter: GenericEdgeStorage<PanelToPresenterEdge>,
    pub panel_to_event_room: GenericEdgeStorage<PanelToEventRoomEdge>,
    pub event_room_to_hotel_room: EventRoomToHotelRoomStorage,
    pub panel_to_panel_type: PanelToPanelTypeStorage,
    pub id_allocators: IdAllocators,
    pub metadata: ScheduleMetadata,
}
```

Update Schedule methods to use trait-based storage instead of generic `add_edge`/`find_related`.

### 3. Update Computed Entity Fields

Update entity computed fields to use new typed edge APIs:

- `Panel.event_room` - use `panel_to_event_room` storage
- `Panel.panel_type` - use `panel_to_panel_type` storage
- `EventRoom.hotel_room` - use `event_room_to_hotel_room` storage
- `HotelRoom.event_rooms` - use `event_room_to_hotel_room` storage (incoming)
- `Presenter.groups` - use `presenter_to_group` storage
- `Presenter.members` - use `presenter_to_group` storage

### 4. Implement EventRoomToHotelRoom Specialized Storage

`EventRoomToHotelRoom` is many-to-one and needs specialized features:
**Data fields:**

- Priority score for selecting best event room
- Time range caching for hotel room usage

**Time range computation algorithm:**

1. Get all panels using each event room (cached lookup)
2. Build time ranges by tracking room switches:
   - Start from convention start hours
   - First range: start → earliest panel start time (different room for same hotel)
   - Subsequent ranges: between panel room switches
   - Last range: final switch → convention end hours
3. Cache computed time ranges per hotel room

**Storage structure:**

```rust
pub struct EventRoomToHotelRoomStorage {
    edges: GenericEdgeStorage<EventRoomToHotelRoomEdge>,
    // Cache: hotel_room_id -> Vec<(time_range_start, time_range_end, event_room_id)>
    time_range_cache: HashMap<EntityId, Vec<(NaiveDateTime, NaiveDateTime, EntityId)>>,
    // Cache: event_room_id -> Vec<panel_id>
    panel_usage_cache: HashMap<EntityId, Vec<EntityId>>,
    cache_invalidation: u64,
}

impl EdgeStorage<EventRoomToHotelRoomEdge> for EventRoomToHotelRoomStorage {
    // Delegate to self.edges for basic operations
    fn add_edge(&mut self, edge: EventRoomToHotelRoomEdge) -> Result<EdgeId, EdgeError> {
        let id = self.edges.add_edge(edge)?;
        self.invalidate_cache();
        Ok(id)
    }
    // ... other trait methods
}
```

### 5. Implement PanelToPresenter Transitive Closures

Add transitive closure support for presenter groups in panel relationships:

**Requirements:**

- `Panel.inclusive_presenters` - all presenters including those via groups
- `Presenter.inclusive_panels` - all panels including those via group membership
- Use `PresenterMemberToGroupStorage` to resolve groups
- Cache transitive results per panel/presenter

**Storage structure:**

```rust
pub struct PanelToPresenterStorage {
    edges: GenericEdgeStorage<PanelToPresenterEdge>,
    // Cache: panel_id -> Vec<presenter_id> (inclusive of groups)
    inclusive_presenters: HashMap<EntityId, Vec<EntityId>>,
    // Cache: presenter_id -> Vec<panel_id> (inclusive of group memberships)
    inclusive_panels: HashMap<EntityId, Vec<EntityId>>,
    cache_invalidation: u64,
}

impl EdgeStorage<PanelToPresenterEdge> for PanelToPresenterStorage {
    // Delegate to self.edges for basic operations
    fn add_edge(&mut self, edge: PanelToPresenterEdge) -> Result<EdgeId, EdgeError> {
        let id = self.edges.add_edge(edge)?;
        self.invalidate_cache();
        Ok(id)
    }
    // ... other trait methods
}
```

### 6. Implement PanelToPanelType Cardinality Constraint

Ensure each panel has exactly one panel type:

**Requirements:**

- Enforce one-to-many: each panel → one type, each type → many panels
- Add validation on edge addition

- Provide `get_panel_type(panel_id)` returning single `Option<EntityId>`
- Provide `get_panels_by_type(type_id)` returning `Vec<EntityId>`

**Storage structure:**

```rust
pub struct PanelToPanelTypeStorage {
    edges: GenericEdgeStorage<PanelToPanelTypeEdge>,
    // Index: panel_id -> type_id (single value, not list)
    panel_to_type_index: HashMap<EntityId, EntityId>,
}

impl EdgeStorage<PanelToPanelTypeEdge> for PanelToPanelTypeStorage {
    fn add_edge(&mut self, edge: PanelToPanelTypeEdge) -> Result<EdgeId, EdgeError> {
        // Enforce one-to-many: remove existing edge for this panel if any
        let panel_id = edge.from_id();
        if let Some(&existing_type_id) = self.panel_to_type_index.get(&panel_id) {
            self.remove_edge_for_panel(panel_id)?;
        }
        let id = self.edges.add_edge(edge)?;
        self.panel_to_type_index.insert(panel_id, edge.to_id());
        Ok(id)
    }
    // ... other trait methods
}
```

**Validation:**

- On `add_edge`: if panel already has a type, remove existing edge before adding new one
- On `remove_edge`: clean up panel_to_type_index

## Acceptance Criteria

- EdgeStorage is a trait with generic `EdgeStorage<E>` for type-safe operations
- Generic `GenericEdgeStorage<E: Edge>` implements `EdgeStorage<E>` for simple edge types
- Specialized storage implementations implement `EdgeStorage<EdgeType>` with relationship-specific behavior
- Schedule uses trait-based edge storage instead of string-based EdgeStorage
- Computed entity fields use new trait-based edge APIs
- EventRoomToHotelRoom implements `EdgeStorage` with time range caching
- PanelToPresenter implements `EdgeStorage` with transitive closures via groups
- PanelToPanelType implements `EdgeStorage` with one-to-many cardinality enforcement
- Old string-based EdgeStorage removed from schedule/storage.rs
- All edge operations are type-safe at compile time via trait bounds
