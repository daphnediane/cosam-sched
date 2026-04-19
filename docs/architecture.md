# Architecture

Overall system architecture, crate layout, and design decisions for the
cosam-sched project.

## Overview

cosam-sched manages the scheduling data for Cosplay America conventions.
The system supports importing schedules from XLSX spreadsheets, editing via
CLI and GUI tools, exporting JSON for the calendar widget, and (eventually)
multi-user offline collaborative editing via CRDT-backed storage.

## Crate Layout

```text
crates/
  schedule-core/   — Entity/field system, data model, schedule container
                     (replaces old schedule-field, schedule-data, schedule-macro)
apps/
  cosam-convert/   — Format conversion (XLSX → JSON, JSON → JSON, etc.)
  cosam-modify/    — CLI schedule editing tool
  cosam-editor/    — GUI desktop editor (GPUI or iced; decision deferred)
```

`schedule-core` is the single library crate for all data model code. Application
crates depend on it and add their own I/O, UI, and format-specific logic.

## Entity / Field System

See `field-system.md` for the full design. See `conversion-and-lookup.md` for the type-safe conversion system including entity resolution support.

Each entity type has three hand-written, visible struct declarations:

| Struct            | Visibility   | Purpose                                                                       |
| ----------------- | ------------ | ----------------------------------------------------------------------------- |
| `<E>CommonData`   | `pub`        | User-facing serializable fields; serde derives                                |
| `<E>InternalData` | `pub(crate)` | `CommonData` + typed UUID + runtime backing (e.g. `time_slot`)                |
| `<E>Data`         | `pub`        | Export/API view: `CommonData` + string code + projected/edge-assembled fields |

`EntityType::InternalData` is what the field system operates on.
`EntityType::Data` is produced by `export(&Schedule)` for serialization and
external APIs.

### Inventory-based field registration

Field descriptors self-register globally via the `inventory` crate. Each field
macro and hand-written descriptor submits a `CollectedField<E>` entry containing
a reference to the static `FieldDescriptor<E>`. The `FieldSet::from_inventory()`
constructor collects all submitted fields for an entity type, sorts them by the
`order: u32` field for stable iteration order, and builds the lookup maps.

This eliminates manual `FieldSet::new(&[...])` lists and prevents accidentally
omitting fields from the registry. Hand-written descriptors use the
`define_field!` macro to bundle the `static` declaration with the required
`inventory::submit!` call.

### Type-level field enums

The system includes type-level mirrors of the runtime value enums:

- `FieldTypeItem` — scalar type tags (String, Text, Integer, Float, Boolean, DateTime, Duration, EntityIdentifier)
- `FieldType` — cardinality wrappers (Single, Optional, List)

These `Copy` enums enable compile-time type declarations and reflection without
requiring runtime values. They are used by converters, importers, and UI code to
determine what type a field expects without calling read/write.

### Entity Types

- **PanelType** — category/kind of panel (prefix, boolean flags, colors)
- **Panel** — a single scheduled event (the primary entity, ~24 stored fields)
- **Presenter** — a person or group credited on panels
- **EventRoom** — a logical room where panels take place
- **HotelRoom** — a physical hotel space that one or more event rooms occupy

## Schedule Container

`Schedule` is the top-level data container. It provides:

- **Two-level type-erased entity store**: `HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>` — one inner map per entity type. This is the single source of truth; there is no separate UUID registry. `identify(uuid)` queries all inner maps via inventory-registered `TypeId` values.
- **`RawEdgeMap`** — a single unified edge store for all relationships (see below).
- **`ScheduleMetadata`** — schedule UUID, timestamps, generator info.

There is no separate `EntityStorage` struct; storage lives directly on `Schedule`.
`get_internal<E>` / `get_internal_mut<E>` / `insert<E>` dispatch via `TypeId`
using the `EntityType::InternalData` associated type.

### Edge Relationships

All entity relationships are stored in a single `RawEdgeMap` on `Schedule`.

`RawEdgeMap` has two fields:

- **`edges`** — `HashMap<NonNilUuid, Vec<RuntimeEntityId>>`: for *heterogeneous* edges
  (different entity types, e.g. Panel → Presenter), both endpoints store each other
  here, making the relationship undirected at the storage level. For *homogeneous*
  edges (same entity type, e.g. Presenter → Presenter groups), forward edges are
  also stored here.
- **`homogeneous_reverse`** — `HashMap<NonNilUuid, Vec<RuntimeEntityId>>`: reverse
  side of homogeneous edges only. Needed to avoid ambiguity: a Presenter UUID in
  `edges` might be both a het back-link (panel) and a homo forward-link (group).

`Schedule` exposes typed generic methods that wrap/unwrap `RuntimeEntityId` ↔
`EntityId<E>`:

- `edges_from::<L, R>(id)` — R entities reachable from `id` via L→R edge
- `edges_to::<L, R>(id)` — L entities pointing to `id`
- `edge_add` / `edge_remove` / `edge_set` — mutators
- `edge_set_to` — reverse-direction bulk set (used for homo reverse fields like `members`)

Het vs homo is determined at runtime by `TypeId::of::<L::InternalData>() == TypeId::of::<R::InternalData>()`.

### Query system

```rust
pub fn find<E: EntityType>(&self, query: &str) -> Vec<(EntityId<E>, MatchPriority)>;
pub fn find_first<E: EntityType>(&self, query: &str) -> Option<EntityId<E>>;
```

Both methods call `E::field_set().match_index(query, data)` for each entity of
type `E`, returning results in `MatchPriority` order. This enables `O(n)`
text search without a separate index. `find_first` is wired as the default
implementation of `EntityType::lookup_by_match_index`.

Tagged presenter lookup (`find_tagged_presenter` / `find_or_create_tagged_presenter`
in `presenter.rs`) uses `find` / `find_first` internally. See
`conversion-and-lookup.md` for the full tagged credit-string format.

Relationship fields exposed via computed fields on node entities:

- **Panel**: `presenters` / `add_presenters` / `remove_presenters`, `event_room`, `panel_type`
- **Presenter**: `groups`, `members`, `inclusive_groups`, `inclusive_members`, `panels`
- **EventRoom**: `hotel_room`, `panels`
- **HotelRoom**: `event_rooms`
- **PanelType**: `panels`

## UUID Identity

All entities are identified by `uuid::NonNilUuid` internally, but most code
should never touch `Uuid` or `NonNilUuid` directly — prefer the typed wrappers:

- **`EntityId<E>`** — typed wrapper for a specific entity type; use in all
  APIs where the entity type is known at compile time
- **`RuntimeEntityId`** — `NonNilUuid` + `&'static str` type name; use for dynamic
  dispatch (e.g. generic commands, serialized references)

### UUID version policy

- **v7** — new entities created at runtime (time-ordered, globally unique)
- **v5** — deterministic identities derived from stable inputs:
  - Edge entities (endpoints are immutable, UUID derived from endpoint pair)
  - Spreadsheet imports: entities that arrive without a UUID use
    `UuidPreference` to derive a stable v5 UUID from the entity type's
    `uuid_namespace()` and a natural key (e.g. presenter name, room name,
    panel Uniq ID), so re-importing the same spreadsheet produces the same UUIDs

`EntityId::from_preference(UuidPreference)` is the primary constructor for new
entities. `EntityId::from_uuid(NonNilUuid)` is `unsafe` — only code with a
UUID→type registry (e.g. `Schedule`) should call it after verifying the type.

### Entity type registry

Entity types self-register globally via the `inventory` crate. Each entity type
submits a `RegisteredEntityType` entry containing its `TYPE_NAME` and UUID namespace
function. The `registered_entity_types()` function iterates all registered types
at runtime, enabling dynamic type lookup without a central enum.

This registry is used by `RuntimeEntityId` deserialization to validate that a
deserialized type name corresponds to a known entity type, rejecting unknown types
as errors.

### Unified ID serialization format

Both `EntityId<E>` and `RuntimeEntityId` serialize to the human-readable string
format `"<type_name>:<uuid>"` (e.g. `"panel:550e8400-e29b-41d4-a716-446655440000"`).
This format is self-describing and consistent between compile-time typed IDs and
runtime dynamic IDs, simplifying debugging and log inspection.

### Avoid bare Uuid / NonNilUuid in APIs

Prefer `EntityId<E>` or `RuntimeEntityId` over raw `Uuid`/`NonNilUuid` in
public function signatures and struct fields. Bare UUID types lose the entity
kind context and bypass compile-time type checking. `NonNilUuid` appears in
internal storage and serialization layers but should not leak into business
logic.

## CRDT Storage (Phase 4)

See `crdt-design.md` for the full design.

`CrdtFieldType` annotations on every field are baked in from Phase 2 onward.
The actual automerge integration is deferred to Phase 4. The field system is
designed so that plugging in CRDT storage requires no changes to entity struct
definitions.

## Application Targets

| App             | Purpose                                                            |
| --------------- | ------------------------------------------------------------------ |
| `cosam-convert` | Batch format conversion: XLSX → internal JSON, JSON → widget JSON  |
| `cosam-modify`  | CLI tool for reading/editing schedule data (search, set, validate) |
| `cosam-editor`  | GUI desktop editor with full schedule management                   |

GUI framework (`iced` vs `GPUI`) decision is deferred to Phase 6.

## Design Decisions

- **No proc-macro for data structs**: `<E>CommonData` and `<E>InternalData`
  declarations are hand-written and visible. Proc-macros and `macro_rules!`
  may be used for boilerplate (trait impls, field accessor singletons, builders)
  but must not obscure the struct definitions.
- **Single library crate**: `schedule-core` replaces the three-crate split
  (`schedule-field` + `schedule-data` + `schedule-macro`) to eliminate the
  layer violations that plagued v10-try3.
- **CRDT-readiness from day one**: `CrdtFieldType` on every field avoids
  retrofit pain when Phase 4 lands.
- **export() over direct serialization**: `<E>InternalData` is `pub(crate)`;
  external code always works with `<E>Data` via `export()`, keeping the
  runtime/internal representation private.
