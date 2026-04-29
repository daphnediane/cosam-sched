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
  schedule-macro/  — Unified field-declaration proc-macro (define_field!)
apps/
  cosam-convert/   — Format conversion (XLSX → JSON, JSON → JSON, etc.)
  cosam-modify/    — CLI schedule editing tool
  cosam-editor/    — GUI desktop editor (GPUI or iced; decision deferred)
```

`schedule-core` is the single library crate for all data model code. Application
crates depend on it and add their own I/O, UI, and format-specific logic.

`schedule-macro` provides a unified `define_field!` function-like proc-macro that
generates `FieldDescriptor` statics for all field types (stored, edge, and custom).
See `field-system.md` for the field declaration syntax.

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

### Builder API

Entity builders provide ergonomic construction with typed setters, UUID assignment,
validation, and rollback semantics (FEATURE-017). The builder system layers on
top of `FieldSet::write_multiple` (FEATURE-046) for atomic batch field updates.

Key components:

- `EntityBuildable` trait — subtrait of `EntityType` for buildable entities
- `build_entity` driver — seeds, populates, validates, and rolls back on failure
- `define_entity_builder!` macro — generates typed builders with `with_*` setters
- Five instantiated builders: `PanelTypeBuilder`, `PanelBuilder`, `PresenterBuilder`,
  `EventRoomBuilder`, `HotelRoomBuilder`

See `field-system.md#builder-system` for details.

### Inventory-based field registration

Field descriptors self-register globally via the `inventory` crate. Each field
macro and hand-written descriptor submits a `CollectedNamedField` entry containing
a `&'static dyn NamedField` reference to the static `FieldDescriptor<E>`. The
`FieldSet::from_inventory()` constructor filters the global registry by entity type
name, downcasts each match to the concrete `FieldDescriptor<E>` type via
`std::any::Any::downcast_ref`, sorts them by the `order: u32` field for stable
iteration order, and builds the lookup maps.

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

`RawEdgeMap` uses a nested structure:

```text
HashMap<NonNilUuid,          // outer key: entity UUID
    HashMap<FieldId,         // inner key: which field on that entity
        Vec<FieldNodeId>>>   // values: (field, uuid) of the other side
```

A `FieldId` is derived from the address of the `&'static dyn NamedField` singleton for
a given field, making it globally unique and stable.  A `FieldNodeId` pairs a `FieldId` with
a `NonNilUuid` to represent "entity X's field Y" as an edge endpoint.

Both directions of every edge are stored symmetrically in the same map.  Homogeneous
(same-type) and heterogeneous (different-type) edges are treated identically — no separate
`homogeneous_reverse` map is needed because each endpoint's field is self-describing.

`Schedule` exposes typed generic methods:

- `connected_entities::<R>(near_node, far_field)` — R entities connected to `near_node` whose far-side field matches `far_field`
- `connected_field_nodes(near_node, far_field_ref)` — `RuntimeFieldNodeId` neighbors filtered by far-side `EdgeRef`
- `edge_add` / `edge_remove` — mutators; accept `impl DynamicFieldNodeId`
- `edge_set<Far>(near, far_field, targets)` — bulk replace neighbors; returns `(added, removed)` diff used for incremental CRDT mirroring

CRDT ownership for any `(near_field, far_field)` pair is resolved by
`edge_crdt::canonical_owner`, which inspects each side's `edge_kind()`:
whichever side carries `EdgeKind::Owner { target_field, .. }` pointing at
the other is the owner. No inventory traversal is required — the owner field
is self-describing through `EdgeDescriptor` (REFACTOR-074). Transitive
(formerly homogeneous) edge mutations also invalidate the `TransitiveEdgeCache`.

### Panel ↔ Presenter Edge Partitions

The Panel ↔ Presenter relationship is split into two independent edge lists
on Panel: `credited_presenters` and `uncredited_presenters`. Each carries
`EdgeKind::Owner { target_field: &FIELD_PANELS, exclusive_with: ... }` so the
macro enforces mutual exclusivity on write. The legacy single `presenters` list
and the `_meta` boolean per-edge map have been removed.

| Panel field                 | Mode       | Semantics                                             |
| --------------------------- | ---------- | ----------------------------------------------------- |
| `credited_presenters`       | read/write | First-class CRDT edge list; owner side                |
| `uncredited_presenters`     | read/write | First-class CRDT edge list; owner side                |
| `presenters`                | read-only  | Derived union of both lists                           |
| `add_credited_presenters`   | write-only | Add to credited; remove from uncredited (exclusivity) |
| `add_uncredited_presenters` | write-only | Add to uncredited; remove from credited (exclusivity) |
| `remove_presenters`         | write-only | Remove from both lists                                |

### Transitive Edge Cache

`TransitiveEdgeCache` (`edge_cache.rs`) caches transitive closures of homogeneous edges
(same entity type on both ends). Heterogeneous-edge transitive queries are not cached
here; they are composed in entity modules from direct `inclusive_edges_from` /
`inclusive_edges_to` calls.

`Schedule` holds the cache as `RefCell<Option<TransitiveEdgeCache>>`. Interior mutability
lets `inclusive_edges_from` / `inclusive_edges_to` update the cache through a `&self`
reference. Setting the field to `None` invalidates the entire cache; it is rebuilt
lazily per-entry on the next query.

```text
homo_edge_cache: RefCell<Option<TransitiveEdgeCache>>
  inclusive_forward: HashMap<NonNilUuid, Box<[NonNilUuid]>>
  inclusive_reverse: HashMap<NonNilUuid, Box<[NonNilUuid]>>
```

The cache key is `NonNilUuid` alone (no type tag) because UUIDs are globally unique
across all entity types — a given UUID belongs to exactly one type, so keying on UUID
alone is sufficient.

**Invalidation:** `homo_edge_cache` is set to `None` inside `edge_add`, `edge_remove`,
`edge_set`, and `remove_entity` whenever the edge is homogeneous.
Heterogeneous-edge mutations do not touch the cache.

**`Schedule` methods:**

- `inclusive_edges_from<L, R>(id)` — all `R` entities transitively reachable from `id`
  via forward homo edges; falls back to direct `edges_from` for het edges.
- `inclusive_edges_to<L, R>(id)` — all `L` entities that transitively point to `id`
  via reverse homo edges; falls back to direct `edges_to` for het edges.

These are the methods used by entity-level computed fields such as `inclusive_groups`
and `inclusive_members` on `Presenter`.

### Query system

Entity lookup is provided by free functions in `schedule-core::lookup`:

```rust
pub fn lookup<E: EntityScannable>(
    schedule: &Schedule,
    query: &str,
    cardinality: FieldCardinality,
) -> Result<Vec<EntityId<E>>, LookupError>;

pub fn lookup_or_create<E: EntityCreatable>(
    schedule: &mut Schedule,
    query: &str,
    cardinality: FieldCardinality,
) -> Result<Vec<EntityId<E>>, LookupError>;

// Convenience helpers:
pub fn lookup_single<E: EntityScannable>(schedule: &Schedule, query: &str)
    -> Result<EntityId<E>, LookupError>;
pub fn lookup_list<E: EntityScannable>(schedule: &Schedule, query: &str)
    -> Result<Vec<EntityId<E>>, LookupError>;
pub fn lookup_or_create_single<E: EntityCreatable>(...)
    -> Result<EntityId<E>, LookupError>;
pub fn lookup_or_create_list<E: EntityCreatable>(...)
    -> Result<Vec<EntityId<E>>, LookupError>;
```

Entity types implement [`EntityMatcher::match_entity`] to own their holistic
match logic (combining any fields they choose), returning a `u8`
[`MatchPriority`] score (`NO_MATCH` = 0 … `EXACT_MATCH` = 255).
[`EntityScannable`] extends `EntityMatcher` with a `scan_entity` hook that
the lookup loop dispatches through — the default implementation performs
the linear scan plus full/partial disambiguation, and entity types can
override it for index-backed lookups or custom token syntax. Types that
support find-or-create additionally implement [`EntityCreatable`] (which
supplies `create_from_string`) and override
[`EntityMatcher::can_create`] to gate creation. This replaced the
previous per-field `IndexableField<E>` / `index_fn` approach.

The lookup algorithm splits queries at `,` / `;`, fast-paths bare and
tagged UUIDs (`"type_name:<uuid>"`), prefers full-string matches over
partial-token matches on ties, enforces cardinality throughout, and defers
creation until after the pre-create cardinality check.

Tagged presenter lookup (`find_tagged_presenter` /
`find_or_create_tagged_presenter` in `presenter.rs`) composes on top of
these primitives. See `conversion-and-lookup.md` for the full tagged
credit-string format.

Relationship fields exposed via computed fields on node entities:

- **Panel**: `credited_presenters` / `uncredited_presenters` (rw, independent edge lists), `presenters` (ro, derived union), `add_credited_presenters` / `add_uncredited_presenters` / `remove_presenters`, `event_rooms` / `add_rooms` / `remove_rooms`, `panel_type`, `credits` (formatted credit strings from `credited_presenters`), `hotel_rooms` (traverses event_rooms to hotel rooms)
- **Presenter**: `groups`, `members`, `inclusive_groups`, `inclusive_members`, `panels` (derived union of credited/uncredited panels), `add_credited_panels` / `add_uncredited_panels` / `remove_panels`
- **EventRoom**: `hotel_rooms`, `panels`
- **HotelRoom**: `event_rooms`
- **PanelType**: `panels`

## UUID Identity

All entities are identified by `uuid::NonNilUuid` internally, but most code
should never touch `Uuid` or `NonNilUuid` directly — prefer the typed wrappers:

- **`EntityId<E>`** — typed wrapper for a specific entity type; use in all
  APIs where the entity type is known at compile time
- **`RuntimeEntityId`** — `NonNilUuid` + `&'static str` type name; use for dynamic
  dispatch (e.g. generic commands, serialized references)

All four ID types (`EntityId<E>`, `RuntimeEntityId`, `FieldNodeId<E>`, `RuntimeFieldNodeId`)
implement a shared trait hierarchy — `EntityUuid` (`.entity_uuid()`), `EntityTyped`
(`.entity_type_name()`), and the blanket `DynamicEntityId`. Field node types additionally
implement `DynamicFieldNodeId` (`.field()`, `.try_as_typed_field<E>()`). This enables APIs
to be generic over any ID type via `impl DynamicEntityId` or `impl DynamicFieldNodeId`.
See `field-system.md#id-trait-hierarchy` for the full hierarchy.

### UUID version policy

- **v7** — new entities created at runtime (time-ordered, globally unique)
- **v5** — deterministic identities derived from stable inputs:
  - Edge entities (endpoints are immutable, UUID derived from endpoint pair)
  - Spreadsheet imports: entities that arrive without a UUID use
    `UuidPreference` to derive a stable v5 UUID from the entity type's
    `uuid_namespace()` and a natural key (e.g. presenter name, room name,
    panel Uniq ID), so re-importing the same spreadsheet produces the same UUIDs

`EntityId::from_preference(UuidPreference)` is the primary constructor for new
entities. `EntityId::new_unchecked(NonNilUuid)` is `unsafe` — only code with a
UUID→type registry (e.g. `Schedule`) should call it after verifying the type.
`EntityId::try_from_dynamic(impl DynamicEntityId)` provides a safe type-checked
conversion from any dynamic ID.

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

## Edit Command System

`schedule-core::edit` provides a command-based mutation API with full undo/redo
support (FEATURE-021). All mutations to the schedule must go through this module.

### `EditCommand`

A `Clone`-able, data-only enum with five variants:

| Variant        | Apply                        | Undo                      |
| -------------- | ---------------------------- | ------------------------- |
| `UpdateField`  | Write new value              | Write stored old value    |
| `AddEntity`    | Build entity from field list | Remove entity             |
| `RemoveEntity` | Remove entity                | Rebuild from snapshot     |
| `MovePanel`    | Wraps a `BatchEdit`          | Inverse `BatchEdit`       |
| `BatchEdit`    | Apply all inner commands     | Apply inverses in reverse |

All variants store only `RuntimeEntityId`, `&'static str` field names, and
`FieldValue` — no closures or `Box<dyn Any>`. This makes `EditCommand: Clone`
and enables serialization for logging and Phase 4 CRDT broadcast.

### Field selection for `AddEntity` / `RemoveEntity`

Only **read+write fields** (`read_fn.is_some() && write_fn.is_some()`) are
included in entity snapshots. This correctly excludes:

- **Read-only** fields (computed queries like `inclusive_panels`) — no `write_fn`.
- **Write-only** modifier fields (`add_presenters`, `remove_presenters`) — no
  `read_fn`. The canonical `presenters` field is read+write and captures the full
  edge state.

### `RegisteredEntityType` fn pointers

Five new function pointers were added to `RegisteredEntityType` for dynamic
dispatch in the edit system:

| Field            | Purpose                                             |
| ---------------- | --------------------------------------------------- |
| `read_field_fn`  | Read a single field by name (captures old value)    |
| `write_field_fn` | Write a single field by name (apply / undo)         |
| `build_fn`       | Build entity from `(name, value)` pairs (redo/undo) |
| `snapshot_fn`    | Snapshot all read+write fields before removal       |
| `remove_fn`      | Remove entity and clear its edges                   |

### `EditHistory`

Two `VecDeque<EditCommand>` stacks (undo / redo) with a configurable
`max_depth` (default 100). `apply` drops the oldest entry when at capacity.
`undo` / `redo` move the top entry between stacks.

### `EditContext`

`EditContext` owns a `Schedule` and an `EditHistory`. It is the sole public
entry point for all schedule mutations:

```rust
ctx.apply(cmd)           // execute + push inverse to undo stack
ctx.undo()               // reverse most recent change
ctx.redo()               // re-apply most recently undone change
ctx.is_dirty() / ctx.mark_clean()  // dirty-state tracking for save prompts
```

Convenience constructors (`update_field_cmd`, `remove_entity_cmd`,
`move_panel_cmd`) capture old values automatically.

### CRDT integration point

`EditContext::apply` is the natural hook for Phase 4: every executed command
can generate CRDT operations to broadcast to peers.

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
  declarations are hand-written and visible. The `schedule-macro` crate provides
  a `define_field!` function-like proc-macro for generating `FieldDescriptor`
  statics, but it does not generate struct definitions.
- **Single library crate**: `schedule-core` replaces the three-crate split
  (`schedule-field` + `schedule-data` + `schedule-macro`) to eliminate the
  layer violations that plagued v10-try3.
- **CRDT-readiness from day one**: `CrdtFieldType` on every field avoids
  retrofit pain when Phase 4 lands.
- **export() over direct serialization**: `<E>InternalData` is `pub(crate)`;
  external code always works with `<E>Data` via `export()`, keeping the
  runtime/internal representation private.
