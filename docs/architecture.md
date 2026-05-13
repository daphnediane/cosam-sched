# Architecture

Overall system architecture, crate layout, and design decisions for the
cosam-sched project.

## Overview

cosam-sched manages the scheduling data for Cosplay America conventions.
The system supports importing schedules from XLSX spreadsheets, editing via
CLI and GUI tools, exporting JSON for the calendar widget, and multi-user
offline collaborative editing via CRDT-backed storage (fully implemented).

## Crate Layout

```text
crates/
  schedule-core/   — Entity/field system, data model, schedule container
  schedule-macro/  — Field and edge descriptor proc-macros
apps/
  cosam-convert/   — Format conversion (XLSX → JSON, JSON → JSON, etc.)
  cosam-modify/    — CLI schedule editing tool
  cosam-editor/    — GUI desktop editor (GPUI or iced; decision deferred)
```

`schedule-core` is the single library crate for all data model code. Application
crates depend on it and add their own I/O, UI, and format-specific logic.

`schedule-macro` provides function-like proc-macros for generating field and edge
descriptors: `accessor_field_properties!`, `edge_field_properties!`, and
`callback_field_properties!`. See `field-system.md` for the field declaration syntax.

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

### Field Descriptors

Fields are declared as static `FieldDescriptor<E>` or `HalfEdgeDescriptor` values:

- **`FieldDescriptor<E>`** - Regular fields (stored, computed, write-only)
- **`HalfEdgeDescriptor`** - Edge relationship fields with ownership metadata

Both implement the `NamedField` trait and are registered globally via the `inventory` crate.

**Important:** Field descriptors must always be accessed via the table module prefix (e.g., `panel::FIELD_NAME`) to avoid name collisions. Never import individual field descriptors directly. See `field-system.md#field-descriptor-usage-guidelines` for detailed usage patterns and rationale.

### Builder API

Entity builders provide ergonomic construction with typed setters, UUID assignment,
validation, and rollback semantics. The builder system layers on top of
`FieldSet::write_multiple` for atomic batch field updates.

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
a `&'static dyn NamedField` reference to the static `FieldDescriptor<E>` or
`HalfEdgeDescriptor`. The `FieldSet::from_inventory()` constructor filters the
global registry by entity type name, downcasts each match to the concrete type
via `std::any::Any::downcast_ref`, sorts them by the `order: u32` field for stable
iteration order, and builds the lookup maps.

This eliminates manual `FieldSet::new(&[...])` lists and prevents accidentally
omitting fields from the registry. All field descriptors must explicitly call
`inventory::submit!` after the singleton definition to register the field.

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
- **Timeline** — a timeline event with a specific time point (not a duration range)

## Schedule Container

`Schedule` is the top-level data container. It provides:

- **Two-level type-erased entity store**: `HashMap<TypeId, HashMap<NonNilUuid, Box<dyn Any + Send + Sync>>>` — one inner map per entity type. This is a cache mirroring the CRDT document.
- **`RawEdgeMap`** — a single unified edge store for all relationships (see below).
- **`TransitiveEdgeCache`** — cache for transitive homogeneous-edge relationships
- **`ScheduleMetadata`** — schedule UUID, timestamps, generator info.
- **Authoritative CRDT document**: `automerge::AutoCommit` — the single source of truth for all entity data

There is no separate `EntityStorage` struct; storage lives directly on `Schedule`.
`get_internal<E>` / `get_internal_mut<E>` / `insert<E>` dispatch via `TypeId`
using the `EntityType::InternalData` associated type.

### CRDT Source of Truth

The authoritative state of every entity lives in the `AutoCommit` document `doc`.
The `entities` HashMap is a cache that mirrors the document: every successful field
write routes through `crdt::write_field` before returning, and `remove_entity`
soft-deletes via the `__deleted` flag. On `load` / `apply_changes` / `merge`
the cache is rebuilt in full from the document.

During load the mirror is disabled via `Schedule::mirror_enabled` so that
rehydrating entities does not generate redundant writes against the doc.

### Edge Relationships

All entity relationships are stored in a single `RawEdgeMap` on `Schedule`.

`RawEdgeMap` uses a nested structure:

```text
HashMap<NonNilUuid,          // outer key: entity UUID
    HashMap<FieldId,         // inner key: which field on that entity
        Vec<RuntimeEntityId>>>   // values: entity UUIDs of connected entities
```

A `FieldId` is derived from the address of the `&'static dyn NamedField` singleton for
a given field, making it globally unique and stable.

Both directions of every edge are stored symmetrically in the same map.
Homogeneous (same-type) and heterogeneous (different-type) edges are treated
identically — no separate `homogeneous_reverse` map is needed because each
endpoint's field is self-describing.

`Schedule` exposes typed generic methods:

- `connected_field_nodes(near_node, far_field)` — entities connected to `near_node` via `far_field`
- `connected_entities<R>(near_node, far_field)` — typed version returning `EntityId<R>`
- `edge_add` / `edge_remove` — mutators; accept `impl DynamicEntityId` and `FullEdge`
- `edge_set<Far>(near, far_field, targets)` — bulk replace neighbors; returns `(added, removed)` diff used for incremental CRDT mirroring
- `inclusive_edges<Near, Far>(near, edge)` — all `Far` entities reachable from `near` via the given edge (transitive for homogeneous edges)

CRDT ownership for any `(near_field, far_field)` pair is resolved by
`crdt::edge::canonical_owner`, which inspects each side's `edge_kind()`:
whichever side carries `EdgeKind::Owner { target_field, .. }` pointing at
the other is the owner. No inventory traversal is required — the owner field
is self-describing through `EdgeKind`. Transitive edge mutations also
invalidate the `TransitiveEdgeCache`.

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

`TransitiveEdgeCache` (`edge/cache.rs`) caches transitive closures of homogeneous edges
(same entity type on both ends). Heterogeneous-edge transitive queries are not cached
here; they are composed in entity modules from direct `connected_field_nodes` calls.

`Schedule` holds the cache as `RefCell<Option<TransitiveEdgeCache>>`. Interior mutability
lets `inclusive_edges` update the cache through a `&self` reference. Setting the field
to `None` invalidates the entire cache; it is rebuilt lazily per-entry on the next query.

```text
transitive_edge_cache: RefCell<Option<TransitiveEdgeCache>>
  cache: HashMap<(FullEdge, NonNilUuid), Box<[NonNilUuid]>>
```

The cache key is `(FullEdge, NonNilUuid)` — the edge encodes traversal direction
(forward and reverse use different `FullEdge` orientations), while the UUID is the
starting node. Multiple independent transitive-edge relationships can share one
cache without key collision.

**Invalidation:** `transitive_edge_cache` is set to `None` inside `edge_add`, `edge_remove`,
`edge_set`, and `remove_entity` whenever the edge is homogeneous.
Heterogeneous-edge mutations do not touch the cache.

**`Schedule` method:**

- `inclusive_edges<Near, Far>(near: EntityId<Near>, edge: FullEdge)` — all `Far` entities
  reachable from `near` via the given edge. When `Near` and `Far` are the same type (homogeneous
  edge), follows edges transitively via the cache; for heterogeneous edges, performs a single-hop
  lookup via `connected_field_nodes`.

This method is used by entity-level computed fields such as `inclusive_groups`
and `inclusive_members` on `Presenter` with explicit `FullEdge` constants (`EDGE_GROUPS`,
`EDGE_MEMBERS`) to avoid near/far confusion.

### Query system

Entity lookup is provided by free functions in `schedule-core::query`:

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

Entity types implement `EntityMatcher::match_entity` to own their holistic
match logic (combining any fields they choose), returning a `u8`
`MatchPriority` score (`NO_MATCH` = 0 … `EXACT_MATCH` = 255).
`EntityScannable` extends `EntityMatcher` with a `scan_entity` hook that
the lookup loop dispatches through — the default implementation performs
the linear scan plus full/partial disambiguation, and entity types can
override it for index-backed lookups or custom token syntax. Types that
support find-or-create additionally implement `EntityCreatable` (which
supplies `create_from_string`) and override
`EntityMatcher::can_create` to gate creation.

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

Both ID types (`EntityId<E>`, `RuntimeEntityId`) implement a shared trait hierarchy —
`EntityUuid` (`.entity_uuid()`), `EntityTyped` (`.entity_type_name()`), and the blanket
`DynamicEntityId`. This enables APIs to be generic over any ID type via `impl DynamicEntityId`.
See `field-system.md#id-trait-hierarchy` for the full hierarchy.

### UUID version policy

- **v7** — new entities created at runtime (time-ordered, globally unique)
- **v5** — deterministic identities derived from stable inputs:
  - Edge entities (endpoints are immutable, UUID derived from endpoint pair)
  - Spreadsheet imports: entities that arrive without a UUID use
    `UuidPreference` to derive a stable v5 UUID from the entity type's
    `uuid_namespace()` and a natural key (e.g. presenter name, room name,
    panel Uniq ID), so re-importing the same spreadsheet produces the same UUIDs

`EntityId::from_preference_unchecked(UuidPreference)` is the unsafe constructor for entities
with specific UUID preferences. For safe conflict-free UUID generation, use `EntityId::generate()`
which creates a new v7 UUID. For safe UUID resolution with conflict checking, use
`Schedule::try_resolve_entity_id()` which handles PreferFromV5 fallback to GenerateNew on conflict.
`EntityId::new_unchecked(NonNilUuid)` is `unsafe` — only code with a UUID→type registry
(e.g. `Schedule`) should call it after verifying the type.
`EntityId::try_from_dynamic(impl DynamicEntityId)` provides a safe type-checked
conversion from any dynamic ID.

### Entity type registry

Entity types self-register globally via the `inventory` crate. Each entity type
submits a `RegisteredEntityType` entry containing its `TYPE_NAME` and UUID namespace
function. The `registered_entity_types()` function iterates all registered types
at runtime, enabling dynamic type lookup without a central enum.

The `registry` module (see `field-system.md#global-registry-registry-module`) wraps
these inventory iterators with `LazyLock`-backed `HashMap` caches. All hot-path
deserialization code uses `registry::get_entity_type` and `registry::get_named_field`
(O(1)) instead of the O(n) iterator forms.

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
support. All mutations to the schedule must go through this module.

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
and enables serialization for logging and CRDT broadcast.

### Field selection for `AddEntity` / `RemoveEntity`

Only **read+write fields** (`read_fn.is_some() && write_fn.is_some()`) are
included in entity snapshots. This correctly excludes:

- **Read-only** fields (computed queries like `inclusive_panels`) — no `write_fn`.
- **Write-only** modifier fields (`add_presenters`, `remove_presenters`) — no
  `read_fn`. The canonical `presenters` field is read+write and captures the full
  edge state.

### `RegisteredEntityType` fn pointers

Five function pointers are added to `RegisteredEntityType` for dynamic
dispatch in the edit system:

| Field            | Purpose                                             |
| ---------------- | --------------------------------------------------- |
| `read_field_fn`  | Read a single field by name (captures old value)    |
| `write_field_fn` | Write a single field by name (apply / undo)         |
| `build_fn`       | Build entity from `(name, value)` pairs (redo/undo) |
| `snapshot_fn`    | Snapshot all read+write fields before removal       |
| `remove_fn`      | Remove entity and clear its edges                   |
| `rehydrate_fn`   | Rebuild entity from CRDT document (load path)       |

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

`EditContext::apply` is the natural hook for CRDT operations: every executed command
generates CRDT operations that are broadcast to peers for sync.

## Sidecar and Extra Fields

The system separates per-entity metadata into three storage tiers:

| Tier                        | Module             | Persisted? | Shared? | Examples                                             |
| --------------------------- | ------------------ | ---------- | ------- | ---------------------------------------------------- |
| `FieldDescriptor` field     | `tables/`          | ✓ CRDT     | ✓       | `av_notes`, `simpletix_link`, `sort_index`           |
| `ExtraFieldDescriptor` data | `extra_field/`     | ✓ CRDT     | ✓       | Declared-but-lightweight columns routed to `__extra` |
| Unknown data extra          | CRDT `__extra` map | ✓ CRDT     | ✓       | Any unrecognized plain-value XLSX column             |
| `ScheduleSidecar`           | `sidecar/`         | ✗          | ✗       | SourceInfo, formula-cell strings, xlsx_sort_key      |

### ScheduleSidecar

`ScheduleSidecar` stores ephemeral per-entity data that must not be persisted to the
CRDT document. It is keyed by `NonNilUuid` and holds:

- **`origin: Option<EntityOrigin>`** — where an entity came from: `Xlsx { file_path, sheet_name, row_index, import_time }` or `Editor { at }`.
- **`formula_extras: HashMap<String, SidecarFormulaField>`** — formula-cell columns captured at import time (formula string + evaluated display value). Used by `update_xlsx` to write back formulas.
- **`xlsx_sort_key: Option<(u32, u32)>`** — the entity's `(col, row)` position in the source XLSX, used during import to assign a normalized `sort_index`.

The sidecar is cleared on `load_from_file` / `load` but is NOT cleared on `save_to_file`.
This allows the same-session `import → edit → save → update_xlsx` workflow without
re-importing the spreadsheet.

### ExtraFieldDescriptor

`ExtraFieldDescriptor` is a lightweight middle tier between a fully-declared `FieldDescriptor`
and a truly unknown column. It registers a canonical name, display label, and optional aliases
for a column that stores its value in the CRDT `__extra` map rather than a dedicated struct field.

Registration uses `inventory::submit!` so extras are declared near the entity type that owns them.
When an `ExtraFieldDescriptor` column earns a proper `FieldDescriptor`, remove the
`ExtraFieldDescriptor`; the `__extra` entry is superseded by the dedicated field path.

### FormulaColumnDef

`FormulaColumnDef` (in `xlsx/columns.rs`) declares known formula columns at the XLSX layer.
These are columns whose cells contain spreadsheet formulas rather than user data, so they
must not be stored in the CRDT. Each entry carries an optional `regenerate` formula template
used during export to always rewrite the formula string regardless of what was in the sidecar.

Currently declared: `Lstart` and `Lend`.

### Import routing priority

When the XLSX importer encounters a column, it routes it through these steps in order:

1. **Known `FieldDescriptor`** — consumed by the field system (includes `FIELD_END_TIME`)
2. **Explicit ignore list** — skipped entirely (`Old Uniq Id`)
3. **`FormulaColumnDef` list** — stored in sidecar `formula_extras`; never in CRDT
4. **`ExtraFieldDescriptor` registry** — value written to CRDT `__extra`
5. **Truly unknown**: formula cell → sidecar; plain value → CRDT `__extra`

### ChangeState tracking

`Schedule` maintains a `change_tracker: HashMap<NonNilUuid, ChangeState>` that records
`Added / Modified / Deleted / Unchanged` for each entity since the last save. The tracker
is reset on `save_to_file`. The sticky-Added rule applies: writing a field on a newly
created entity does not downgrade it from `Added` to `Modified`. See `crdt-design.md`
for the full transition table.

## CRDT Storage

See `crdt-design.md` for the full design.

The CRDT system is fully implemented using automerge. The `AutoCommit` document
is the single source of truth; the in-memory HashMap cache mirrors it.
Edge relationships are stored as owner-list fields in the CRDT document.

## Application Targets

| App             | Purpose                                                            |
| --------------- | ------------------------------------------------------------------ |
| `cosam-convert` | Batch format conversion: XLSX → internal JSON, JSON → widget JSON  |
| `cosam-modify`  | CLI tool for reading/editing schedule data (search, set, validate) |
| `cosam-editor`  | GUI desktop editor with full schedule management                   |

GUI framework (`iced` vs `GPUI`) decision is deferred.

## Design Decisions

- **No proc-macro for data structs**: `<E>CommonData` and `<E>InternalData`
  declarations are hand-written and visible. The `schedule-macro` crate provides
  function-like proc-macros for generating `FieldDescriptor` statics, but it does
  not generate struct definitions.
- **Single library crate**: `schedule-core` replaces the three-crate split
  (`schedule-field` + `schedule-data` + `schedule-macro`) to eliminate the
  layer violations that plagued v10-try3.
- **CRDT from day one**: `CrdtFieldType` on every field and automerge document
  as source of truth avoids retrofit pain.
- **export() over direct serialization**: `<E>InternalData` is `pub(crate)`;
  external code always works with `<E>Data` via `export()`, keeping the
  runtime/internal representation private.
- **Owner-list edge storage**: Edge relationships are stored as lists on canonical
  owner entities in the CRDT document, enabling add-wins semantics for concurrent
  edge mutations.
