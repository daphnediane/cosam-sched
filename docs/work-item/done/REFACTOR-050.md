# Migrate Entity IDs and Storage to NonNilUuid with TypedId/EntityUUID/TypedStorage

## Summary

Aggressive migration to `NonNilUuid` as the primary UUID type throughout schedule-data,
with `TypedId`, `EntityUUID`, and `TypedStorage` traits for generic, zero-dispatch entity access.

## Status

Completed

## Priority

High

## Description

Implements "Option B" from `docs/NonNilUuid_Refactor_Plan.md` and the design from
`docs/EntityUUID_and_TypedId.md`. `NonNilUuid` is now the default throughout the codebase;
raw `Uuid` is only used at system boundaries (deserialization, UUID generation).

### Core Philosophy

- **NonNilUuid is the default** — all entity IDs, edge relationships, and internal APIs
- **Uuid is for boundaries only** — JSON deserialization (nil rejection), `Uuid::now_v7()` generation
- **Simplified naming** — `from_uuid(NonNilUuid)` is the primary constructor; `try_from_raw_uuid(Uuid)` for boundaries

## Implementation Details

### Entity ID Types

All five ID newtypes (`PanelId`, `PresenterId`, `EventRoomId`, `HotelRoomId`, `PanelTypeId`) now:

- Wrap `NonNilUuid` internally
- Implement `From<NonNilUuid>`, `From<XxxId> -> NonNilUuid`, `From<XxxId> -> Uuid`
- Provide `non_nil_uuid() -> NonNilUuid`, `uuid() -> Uuid`, `from_uuid(NonNilUuid) -> Self`
- Provide `try_from_raw_uuid(Uuid) -> Option<Self>` for deserialization boundaries

### TypedId Trait

Uniform interface for all entity ID wrappers:

```rust
pub trait TypedId: Copy + Clone + Send + Sync + fmt::Debug + 'static {
    type EntityType: EntityType;
    fn non_nil_uuid(&self) -> NonNilUuid;
    fn uuid(&self) -> Uuid { self.non_nil_uuid().into() }
    fn kind() -> EntityKind { Self::EntityType::KIND }
    fn from_uuid(uuid: NonNilUuid) -> Self;
    fn try_from_raw_uuid(uuid: Uuid) -> Option<Self> { NonNilUuid::new(uuid).map(Self::from_uuid) }
}
```

### EntityUUID Enum

`NonNilUuid` tagged with entity kind at runtime — returned by `Schedule::identify()`:

```rust
pub enum EntityUUID {
    Panel(PanelId),
    Presenter(PresenterId),
    EventRoom(EventRoomId),
    HotelRoom(HotelRoomId),
    PanelType(PanelTypeId),
}
```

### TypedStorage Trait

On `EntityType` marker structs; provides access to the concrete `HashMap` for that type:

```rust
pub trait TypedStorage: EntityType {
    fn typed_map(storage: &EntityStorage) -> &HashMap<NonNilUuid, Self::Data>;
    fn typed_map_mut(storage: &mut EntityStorage) -> &mut HashMap<NonNilUuid, Self::Data>;
}
```

### Real EntityStorage

Replaced the debug-string stub with concrete typed maps:

```rust
pub struct EntityStorage {
    pub panels: HashMap<NonNilUuid, PanelData>,
    pub presenters: HashMap<NonNilUuid, PresenterData>,
    pub event_rooms: HashMap<NonNilUuid, EventRoomData>,
    pub hotel_rooms: HashMap<NonNilUuid, HotelRoomData>,
    pub panel_types: HashMap<NonNilUuid, PanelTypeData>,
}
```

All `EntityStorage` methods (`get`, `get_by_uuid`, `add_with_uuid`, `contains_uuid`,
`update`, `find`, `get_many`, `get_by_index`) work against real typed maps.

### Schedule Methods

- `identify(uuid) -> Option<EntityUUID>` — registry lookup returning typed UUID
- `fetch_entity<Id: TypedId>(id) -> Option<&Data>` — zero-dispatch borrow of internal data
- `fetch_typed<Id: TypedId>(id) -> Option<PublicEntityRef>` — owned public value
- `lookup_typed<Id: TypedId>(id) -> Option<EntityRef>` — borrowed enum value
- `fetch_uuid(uuid) -> Option<PublicEntityRef>` — via `identify()` dispatch
- `lookup_uuid(uuid) -> Option<EntityRef>` — via `identify()` dispatch
- All `Schedule` generics updated to `T: TypedStorage` bound

### Edge System

All edge storages migrated from `Uuid` to `NonNilUuid`:

- `Edge` trait: `from_uuid()`/`to_uuid()` return `Option<NonNilUuid>`
- `EdgeStorage` trait: `find_outgoing`/`find_incoming`/`edge_exists` take `NonNilUuid`
- `GenericEdgeStorage` indices use `NonNilUuid`
- `PanelToPresenterStorage`, `PresenterToGroupStorage`, `PanelToPanelTypeStorage`,
  `EventRoomToHotelRoomStorage` all migrated

### Query Layer

`Finder<T>` and `Updater<T>` both bounded by `TypedStorage + Sized` throughout.

## Acceptance Criteria

- ✅ All entity ID types wrap `NonNilUuid`
- ✅ `TypedId` trait implemented for all five ID types
- ✅ `EntityUUID` enum and `Schedule::identify()` working
- ✅ `EntityStorage` stores and retrieves real data (no stub)
- ✅ `TypedStorage` trait on all five `EntityType` marker structs
- ✅ All edge storage uses `NonNilUuid` keys
- ✅ `fetch_entity`, `fetch_typed`, `lookup_typed`, `fetch_uuid`, `lookup_uuid` functional
- ✅ 101 tests passing

## Notes

- `NonNilUuid` is the standard; `from_uuid(NonNilUuid)` is the primary constructor name
  since the verbose `from_non_nil_uuid` is unnecessary when the whole system uses NonNilUuid
- UUID v7 (time-ordered) was chosen over v4 (random) for better index locality
- Raw `Uuid` is kept for `try_from_raw_uuid` at deserialization boundaries and `Uuid::now_v7()` generation
- This work replaces the design documents `docs/EntityUUID_and_TypedId.md` and
  `docs/NonNilUuid_Refactor_Plan.md`, which were planning artifacts

## Dependencies

- See also: REFACTOR-037 (parent UUID migration plan), REFACTOR-049 (remaining tests)
