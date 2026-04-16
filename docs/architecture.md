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

See `field-system.md` for the full design.

Each entity type has three hand-written, visible struct declarations:

| Struct            | Visibility   | Purpose                                                                       |
| ----------------- | ------------ | ----------------------------------------------------------------------------- |
| `<E>CommonData`   | `pub`        | User-facing serializable fields; serde derives                                |
| `<E>InternalData` | `pub(crate)` | `CommonData` + typed UUID + runtime backing (e.g. `time_slot`)                |
| `<E>Data`         | `pub`        | Export/API view: `CommonData` + string code + projected/edge-assembled fields |

`EntityType::InternalData` is what the field system operates on.
`EntityType::Data` is produced by `export(&Schedule)` for serialization and
external APIs.

### Entity Types

- **PanelType** — category/kind of panel (prefix, boolean flags, colors)
- **Panel** — a single scheduled event (the primary entity, ~24 stored fields)
- **Presenter** — a person or group credited on panels
- **EventRoom** — a logical room where panels take place
- **HotelRoom** — a physical hotel space that one or more event rooms occupy

## Schedule Container

`Schedule` is a **coordination proxy**, not a data owner. It provides:

- UUID registry mapping `NonNilUuid` → `EntityKind`
- `EntityStorage` — a struct of typed `HashMap<Id, InternalData>` maps, one per entity type
- Edge index maps (e.g. `panel_to_presenters`, `event_room_to_hotel_room`)
- Unified `add_entity()` / `add_edge()` / `export_entity()` API

Entity types own their storage conceptually. `Schedule` dispatches to the
right storage map via `TypedStorage` / `TypedEdgeStorage` traits.

### Edge Relationships

Relationships between entities are stored as edge maps in `EntityStorage`,
not as redundant fields on the entity data. Relationship IDs appear in
`<E>Data` (the export struct) assembled during `export()`, not in
`<E>InternalData` (with the exception of forward-side backing vecs where
needed for computed field writes).

Primary relationship fields exposed via computed fields on node entities:

- **Panel**: `presenters` / `add_presenters` / `remove_presenters`, `event_rooms`, `panel_type`
- **Presenter**: `groups`, `members`, `panels`
- **EventRoom**: `hotel_rooms`, `panels`
- **HotelRoom**: `event_rooms`

## UUID Identity

All entities are identified by `uuid::NonNilUuid` internally, but most code
should never touch `Uuid` or `NonNilUuid` directly — prefer the typed wrappers:

- **`EntityId<E>`** — typed wrapper for a specific entity type; use in all
  APIs where the entity type is known at compile time
- **`RuntimeEntityId`** — `NonNilUuid` + `EntityKind`; use for dynamic
  dispatch (e.g. generic commands, serialized references)

### UUID version policy

- **v7** — new entities created at runtime (time-ordered, globally unique)
- **v5** — deterministic identities derived from stable inputs:
  - Edge entities (endpoints are immutable, UUID derived from endpoint pair)
  - Spreadsheet imports: entities that arrive without a UUID use
    `UuidPreference` to derive a stable v5 UUID from the entity's natural
    key (e.g. presenter name, room name, panel Uniq ID), so re-importing the
    same spreadsheet produces the same UUIDs

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
