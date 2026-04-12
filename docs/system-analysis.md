# cosam-sched System Analysis

Reference document for AI-assisted sessions. Describes the current state of
`cosam_sched` (active repo), source-of-truth context from related repos, and
the design decisions made so far. Update as each META-026 work item completes.

---

## 1. Repository Map

| Repo path           | Branch                   | Purpose                                                                                                                                                | Trust level                                         |
| ------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------- |
| `main/`             | feature/rewrite          | **Active workspace** — Phase 2 of META-001 in progress                                                                                                 | Canonical                                           |
| `v10-try-2/`        | feature/json-v10-try2    | v10 experiment retry: edge system + query engine, full entity definitions                                                                              | Reference (partially superseded)                    |
| `v10-try-1/`        | feature/json-v10-try1    | v10 development sketch: schedule-data along side updated schedule-core                                                                                 | Reference (design ideas only; largely superseded)   |
| `v9/`               | release/schedule-core-v9 | schedule-core with xlsx import/export and GPUI editor shell                                                                                            | Reference (mostly out-of-date; but more functional) |
| `schedule-to-html/` | (different repo)         | Perl static pipeline: reads spreadsheet → generates HTML/JSON for widget display. **No editing capability** — schedule data lived in spreadsheet only. | Spreadsheet format authority                        |

> **Note on `schedule-to-html`:** This predecessor was intentionally read-only. It consumed a manually-maintained spreadsheet (the authoritative source) and produced static HTML pages and a JavaScript widget JSON blob. There was no concept of an in-app data model, editing, undo, or JSON round-tripping. The current Rust rewrite exists specifically to add those capabilities.

---

## 2. Active Work: META-001 / META-026

**META-001** is the top-level architecture redesign (CRDT-backed schedule system).
Phases track in META-025 through META-031.

**META-026 (Phase 2 — Core Data Model)** is currently *In Progress*.
Sub-items:

| Item        | Title                                                   | Status       |
| ----------- | ------------------------------------------------------- | ------------ |
| FEATURE-003 | EntityFields derive macro (schedule-macro)              | Largely done |
| FEATURE-004 | Field system (traits, FieldValue, FieldSet, validation) | Largely done |
| FEATURE-005 | Core entity definitions                                 | Largely done |
| FEATURE-006 | UUID-based identity and typed ID wrappers               | Done         |
| FEATURE-007 | Edge/relationship system                                | Largely done |
| FEATURE-008 | Schedule container and EntityStorage                    | Largely done |
| FEATURE-009 | Query system                                            | Partial      |
| FEATURE-010 | Edit command system with undo/redo history              | Not started  |

---

## 3. Crate Structure (`cosam_sched`)

```text
cosam_sched/
├── crates/
│   ├── schedule-data/    # Core data model
│   └── schedule-macro/   # EntityFields proc-macro
└── apps/
    ├── cosam-convert/    # Format conversion CLI
    ├── cosam-modify/     # CLI editing tool
    └── cosam-editor/     # GUI editor (GPUI or iced, decision deferred to Phase 6)
```

---

## 4. Entity Model

### 4.1 Node Entities

| Entity      | Key fields                                                                                                                                                | Indexable fields                     |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| `Panel`     | `uid` (Uniq ID), `name`, `time_slot` (TimeRange), cost flags, workshop fields                                                                             | `uid` (220), `name` (210)            |
| `Presenter` | `name`, `rank` (`PresenterRank`: Guest/Judge/Staff/InvitedGuest/Panelist/FanPanelist), `sort_rank`, `is_group`, `always_grouped`, `always_shown_in_group` | `name` (200)                         |
| `EventRoom` | `room_name`, `long_name`, `sort_key`                                                                                                                      | `room_name` (220), `long_name` (210) |
| `HotelRoom` | `room_name`, `long_name`                                                                                                                                  | (same pattern)                       |
| `PanelType` | `prefix` (2-letter), `panel_kind`, boolean flags (`is_workshop`, `is_break`, `is_cafe`, `is_private`, `is_timeline`, …), `color`, `bw`                    | `prefix` (220), `panel_kind` (210)   |

### 4.2 Relationships (Virtual Edges)

Relationships are stored as **UUID fields directly on the owning entity** —
there are no separate edge entities and no edge UUIDs.  `EntityStorage`
maintains per-relationship reverse lookup indexes so queries from either side
remain efficient.

| Relationship          | Owning entity | Stored field(s)                   | Reverse index in EntityStorage |
| --------------------- | ------------- | --------------------------------- | ------------------------------ |
| Panel → PanelType     | `Panel`       | `panel_type: Option<PanelTypeId>` | `panels_by_panel_type`         |
| Panel → EventRoom     | `Panel`       | `event_room: Option<EventRoomId>` | `panels_by_event_room`         |
| Panel → Presenter(s)  | `Panel`       | `presenters: Vec<PresenterId>`    | `panels_by_presenter`          |
| EventRoom → HotelRoom | `EventRoom`   | `hotel_rooms: Vec<HotelRoomId>`   | `event_rooms_by_hotel_room`    |
| Presenter → Group(s)  | `Presenter`   | `groups: Vec<PresenterId>`        | `presenters_by_group`          |

`Presenter` also carries `is_explicit_group: bool` (set when a presenter is
explicitly declared a group, as opposed to implicitly acting as one because
others point to it), `always_grouped: bool`, and `always_shown_in_group: bool`
(entity-level flags matching old `schedule-to-html` behavior;
per-membership-edge granularity is deferred — see IDEA-039).

### 4.3 Key Design Decisions

- **Virtual edges (relationships as owned fields)**: each relationship is a
  UUID field (or `Vec<TypedId>`) stored directly on the owning entity — no
  separate edge entities, no edge UUIDs.  Removing a relationship is a field
  mutation on the owning entity, not an entity deletion.  Entities themselves
  use soft deletion.
- **Reverse lookup indexes**: `EntityStorage` maintains one
  `HashMap<NonNilUuid, Vec<NonNilUuid>>` per relationship for the non-owning
  side (e.g., `panels_by_presenter`).  These are updated by entity type
  `on_insert` / `on_remove` / `on_update` hooks, not by a separate `add_edge`
  call-site.
- **Schedule is a proxy**: `Schedule` provides UUID registry coordination
  and a unified API, but entity types own their storage access patterns.
  Computed field closures access `EntityStorage` directly, not through
  `Schedule` convenience methods.
- **`<Type>EntityType` owns the logic**: All non-trivial implementations that
  work with entity or edge data belong as methods on the relevant
  `<Type>EntityType` struct. `Schedule` methods and computed field closures
  are **thin adapters** — they call the `EntityType` method and return the
  result, containing no logic of their own. See §10 for examples.
- **Rooms split**: `EventRoom` (logical schedule room) vs `HotelRoom` (physical
  hotel space). One event room can map to multiple hotel rooms; the
  `hotel_rooms` field on `EventRoom` holds the list.  Time-partitioned mapping
  (different physical rooms at different times) is deferred.

---

## 5. Identifier System

### TypedId trait

Every entity type has a newtype wrapper around `NonNilUuid`:

```rust
pub struct PanelId(NonNilUuid);
pub struct PresenterId(NonNilUuid);
// etc.
```

Generated by the macro. Each wrapper:

- Implements `TypedId` (provides `.non_nil_uuid()`, `.uuid()`, `.kind()`,
  `from_uuid()`, `try_from_raw_uuid()`)
- Has a kebab-prefixed `Display`: `"panel-<uuid>"`, `"presenter-<uuid>"`, etc.
- Serializes/deserializes as a plain UUID string (no prefix in JSON)
- Has a per-type UUID namespace (`cosam.<snake_case_name>` hashed with v5 DNS)

### UuidPreference

Controls UUID assignment in builders:

| Variant           | When to use                                                     |
| ----------------- | --------------------------------------------------------------- |
| `GenerateNew`     | New entity with no natural key (default, emits v7)              |
| `FromV5 { name }` | Import from spreadsheet — deterministic from natural key string |
| `Exact(uuid)`     | Restoring from serialized state                                 |

### EntityUUID / EntityKind

`EntityUUID` is a tagged enum (one variant per entity kind) returned by
`Schedule::identify()`. `EntityKind` is a plain enum used in the UUID registry.

---

## 6. Macro System (`schedule-macro`)

`#[derive(EntityFields)]` generates a complete data model from struct definitions:

| Generated Item         | Purpose                                             |
| ---------------------- | --------------------------------------------------- |
| `<Name>Data`           | Storage struct (stored fields only)                 |
| `<Name>EntityType`     | Type metadata, field registry, validation           |
| `<Name>Id`             | Typed UUID wrapper (`TypedId` impl)                 |
| `<Name>Builder`        | Construction with validation and schedule insertion |
| Per-field unit structs | `NamedField` impls for field access                 |
| `fields` module        | Public constants for all field structs              |

**Design rationale**: The macro separates the user-facing struct (with computed
fields as typed accessors) from the storage struct (`Data`). This allows computed
fields to access the schedule or other entities while maintaining clean serialization.

**See `field-system.md`** for complete macro attribute reference, closure syntax,
and usage patterns.

---

## 7. Field System

The field system provides type-safe field access with three key abstractions:

| Component        | Purpose                    | Key Types                                                   |
| ---------------- | -------------------------- | ----------------------------------------------------------- |
| **FieldValue**   | Universal runtime value    | `String`, `Integer(i64)`, `NonNilUuid`, `List`, `Map`, etc. |
| **FieldSet**     | Per-entity static registry | `get_field()`, `match_index()`, required/indexable tracking |
| **Field Traits** | Type-safe access patterns  | `ReadableField`, `WritableField`, `IndexableField`          |

**Trait hierarchy** (blanket impls auto-promote `Simple*` traits):

```text
NamedField
├── SimpleReadableField<T> → ReadableField<T>
├── SimpleWritableField<T> → WritableField<T>
├── IndexableField<T>      → match_field() for lookups
```

Computed fields use schedule-aware variants (`&Schedule` parameter) for edge
access and mutations. Match priority levels: `EXACT_MATCH=255` down to `NO_MATCH=0`.

**See `field-system.md`** for complete trait documentation, `FieldValue` conversions,
and field usage patterns.

---

## 8. Panel Uniq ID Parsing

`PanelUniqId` parses the spreadsheet "Uniq ID" string:

```text
<PREFIX><NUM>[P<part>][S<session>][<suffix>]
```

- Prefix normalised to 2 uppercase letters (`SPLIT` → `SP`, `BREAK` → `BR`)
- Provides `base_id()` (`"GW097"`), `full_id()`, `part_id()`
- Stored in `PanelData.parsed_uid: Option<PanelUniqId>`

---

## 9. Spreadsheet Format Summary

Source: `cosam-data-old/docs/spreadsheet-format.md`

### Schedule Sheet Columns (current, 2024–2026)

Core: `Uniq ID`, `Name`, `Room`, `Start Time`, `Duration`, `Description`,
`Prereq`, `Note`, `Notes (Non Printing)`, `Workshop Notes`, `Power Needs`,
`Sewing Machines`, `AV Notes`, `Difficulty`, `Cost`, `Seats Sold`, `PreReg Max`,
`Capacity`, `Have Ticket Image`, `SimpleTix Event`, `Ticket Sale`,
`Hide Panelist`, `Alt Panelist`

Ignored (internal): `Old Uniq Id`, `Lstart`, `Lend`

### Presenter Columns (tagged format, 2022–present)

Header syntax `Kind:Name=Group` or `Kind:Other`:

- Kinds: `G` (guest), `J` (judge), `S` (staff), `I` (invited), `P` (panelist), `F` (fan panelist)
- `G:Name` — individual; cell is a presence flag
- `G:Name=Group` — member of group (shown individually or as group)
- `G:Name==Group` — sets `always_shown` on the *group*
- `G:<Name=Group` — sets `always_grouped` on the *individual*
- `G:Other` — cell is comma-separated list of additional names

Legacy format (2016–2019) uses group-header columns (`Guests:`, `Staff:`, etc.)
followed by per-name columns without kind prefix.

### Rooms Sheet

`Room Name` (matches Schedule.Room), `Long Name`, `Hotel Room`, `Sort Key`
(≥ 100 = hidden).

### PanelTypes Sheet

`Prefix`, `Panel Kind`, `Hidden`, `Is Workshop`, `Is Break`, `Is Café`,
`Is Room Hours`, `Is TimeLine`, `Is Private`, `Color`, `BW`

---

## 10. Schedule Container (FEATURE-008, revised by REFACTOR-036/037/038)

### Structure

```text
Schedule
├── entities: EntityStorage — per-type HashMap<NonNilUuid, Data>
│   ├── panels: HashMap<NonNilUuid, PanelData>
│   ├── presenters: HashMap<NonNilUuid, PresenterData>
│   ├── event_rooms: HashMap<NonNilUuid, EventRoomData>
│   ├── hotel_rooms: HashMap<NonNilUuid, HotelRoomData>
│   ├── panel_types: HashMap<NonNilUuid, PanelTypeData>
│   │
│   └── Reverse relationship indexes (maintained by entity type hooks)
│       ├── panels_by_panel_type:   HashMap<NonNilUuid, Vec<NonNilUuid>>
│       ├── panels_by_event_room:   HashMap<NonNilUuid, Vec<NonNilUuid>>
│       ├── panels_by_presenter:    HashMap<NonNilUuid, Vec<NonNilUuid>>
│       ├── event_rooms_by_hotel_room: HashMap<NonNilUuid, Vec<NonNilUuid>>
│       └── presenters_by_group:    HashMap<NonNilUuid, Vec<NonNilUuid>>
├── uuid_registry: HashMap<NonNilUuid, EntityKind>
└── metadata: ScheduleMetadata
```

### `EntityStore<T>` Trait

All entity CRUD goes through a single generic trait instead of per-type methods:

```rust
pub trait EntityStore<T: EntityType> {
    fn get_entity(&self, uuid: NonNilUuid) -> Option<&T::Data>;
    fn get_entity_mut(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data>;
    fn insert_entity(&mut self, uuid: NonNilUuid, data: T::Data) -> Result<(), InsertError>;
    fn remove_entity(&mut self, uuid: NonNilUuid) -> Option<T::Data>;
    fn contains_entity(&self, uuid: NonNilUuid) -> bool;
}
```

- **`EntityStorage`** implements `EntityStore<T>` for all `T: TypedStorage` via
  a blanket impl. `TypedStorage` maps each entity type to its `HashMap` field.
- **`Schedule`** also implements `EntityStore<T>`, adding UUID registry management
  on top of the raw storage operations.
- Convenience methods on `Schedule`: `add_entity::<T>(data)` → `Result<T::Id, InsertError>`,
  `get_entity::<T>(id)`, `get_entity_mut::<T>(id)`, `remove_entity::<T>(id)`,
  `contains_entity::<T>(id)`, `get_entity_by_uuid::<T>(uuid)`.

### EntityType Trait

```rust
pub trait EntityType: 'static + Send + Sync + Debug {
    type Data: InternalData;
    type Id: TypedId<EntityType = Self>;
    const TYPE_NAME: &'static str;
    const KIND: EntityKind;
    fn field_set() -> &'static FieldSet<Self>;
    fn validate(data: &Self::Data) -> Result<(), ValidationError>;
    /// Called after entity is inserted into its HashMap.
    fn on_insert(storage: &mut EntityStorage, data: &Self::Data) {}
    /// Called before entity is removed from its HashMap.
    fn on_remove(storage: &mut EntityStorage, data: &Self::Data) {}
    /// Called when entity data changes in place (field update).
    fn on_update(storage: &mut EntityStorage, old: &Self::Data, new: &Self::Data) {}
}
```

The `type Id` associated type links each entity type to its typed ID wrapper,
enabling generic methods like `add_entity` to return the correct ID type.

`PanelEntityType`, `EventRoomEntityType`, and `PresenterEntityType` implement
`on_insert` / `on_remove` / `on_update` to maintain their respective reverse
relationship indexes in `EntityStorage`.

### Builder → Schedule Integration

`Builder::build(&mut Schedule)` validates required fields, resolves the UUID
(via `UuidPreference`), constructs the data struct, and inserts it into the
schedule in one step. Returns `Result<TypedId, BuildError>` where `BuildError`
combines `ValidationError` and `InsertError`.

`Builder::build_data()` produces the data struct without inserting (useful for
tests or deferred insertion).

### Relationship Convenience Methods on EntityType

Each owning entity type provides static convenience query methods on
`EntityStorage` — forward lookups read directly from entity data; reverse
lookups use the index:

| EntityType            | Forward (reads entity field)                      | Reverse (reads index)                                     |
| --------------------- | ------------------------------------------------- | --------------------------------------------------------- |
| `PanelEntityType`     | `panel_type_of`, `event_room_of`, `presenters_of` | `panels_of_type`, `panels_in_room`, `panels_of_presenter` |
| `EventRoomEntityType` | `hotel_rooms_of`                                  | `event_rooms_in_hotel_room`                               |
| `PresenterEntityType` | `groups_of`, `is_group`                           | `members_of`, `is_explicit_group`                         |

These take `&EntityStorage` (not `&Schedule`); **all logic lives here**.
`Schedule` has thin wrapper methods that delegate and add no logic.

### Membership Mutation Helpers

`Schedule` provides convenience methods for managing presenter group membership
via field mutations on `PresenterData`:

| Method                          | Effect                                        |
| ------------------------------- | --------------------------------------------- |
| `set_presenter_group(id, bool)` | Set `is_explicit_group` flag                  |
| `add_member(member, group)`     | Push `group` to `member.groups`; update index |
| `remove_member(member, group)`  | Remove from `member.groups`; update index     |

### Presenter Tag-String Lookup

`PresenterEntityType::lookup_tagged(schedule: &mut Schedule, input: &str) -> Result<PresenterId, LookupError>`
is the **implementation** in `entity/presenter.rs`.  `Schedule::lookup_tagged_presenter(input)`
is a **thin delegate** that calls it and returns the result — it contains no
additional logic.  Handles UUID references, tagged credit strings, and bare
name lookups.  Documented fully in FEATURE-009.

`PresenterEntityType::find_or_create_by_name(schedule, name, rank)` is a public
helper for callers that already know the name and rank directly.

`LookupError`: `Empty`, `UuidNotFound`, `InvalidUuid`, `NameNotFound`,
`UnknownTag`, `OtherSentinel`.

### Not Yet Implemented

- **Transitive presenter-to-group closure cache** (BFS over `groups` Vec)
- **Entity name lookup** (`get_entity_names`) — stub exists
- **Soft delete** marker on entities

---

## 11. Edit System (FEATURE-010, not yet implemented)

Planned `EditCommand` enum wrapping reversible operations:
`UpdateField`, `AddEntity`, `RemoveEntity`, `MovePanel`, `BatchEdit`.

Relationship changes (adding/removing presenters, setting a room, etc.) go
through `UpdateField` on the owning entity — no separate `AddEdge`/`RemoveEdge`
commands are needed since relationships are stored as fields.

`EditHistory` — stack-based undo/redo with configurable max depth.

`EditContext` = `Schedule` + `EditHistory`. All public mutations go through it.

This is also the **CRDT integration point** (Phase 3 / META-027): applied
commands can emit CRDT operations for peer broadcast.

---

## 12. Presenter Group Semantics

Groups are `Presenter` entities where either `is_explicit_group = true` (the
presenter was declared a group in the data) or where other presenters list the
presenter in their `groups: Vec<PresenterId>` field (implicit group).  There
is no longer a self-loop edge to mark group status.

Membership is encoded as `groups: Vec<PresenterId>` stored directly on the
member `Presenter`.  The reverse index `presenters_by_group` in `EntityStorage`
enables efficient lookup of all members of a given group.

`always_grouped: bool` and `always_shown_in_group: bool` are entity-level
fields on `Presenter` (apply to all their group memberships).  Per-membership
granularity is deferred to IDEA-039.

### Nested Groups and Transitive Membership

Groups can be members of other groups (group-of-groups). Example:

```text
Alice ──member──▶ Team A ──member──▶ Department
Bob   ──member──▶ Team A
Carol ──member──▶ Team B ──member──▶ Department
```

This creates two levels of membership:

- **Direct members of Team A**: {Alice, Bob}
- **Direct members of Team B**: {Carol}
- **Direct members of Department**: {Team A, Team B}
- **Inclusive members of Department**: {Team A, Team B, Alice, Bob, Carol}
- **Inclusive groups of Alice**: {Team A, Department}
- **Inclusive groups of Carol**: {Team B, Department}

Note that group membership chains are **directional** — Alice belongs to Team A
which belongs to Department, but Alice does *not* belong to Team B, and Bob
does *not* belong to Department through any path other than Team A.

Nested groups haven't been used in recent convention years but the model
supports them and the implementation must handle them correctly.

### Panel → Presenter Transitive Inclusion

A panel's **direct presenters** are the `Presenter` UUIDs in
`PanelData.presenters: Vec<PresenterId>`. The **transitive presenter set** for a panel is the
union of, for each direct presenter P:

1. P itself
2. All **inclusive groups** of P (upward: P's groups, their groups, etc.)
3. All **inclusive members** of P if P is a group (downward: P's members,
   their members, etc.)

This means the transitive set expands **both upward and downward** from each
direct presenter.

**Worked examples** (using the group graph above):

A panel directly hosted by **Alice**:

- Direct presenters: {Alice}
- Alice's inclusive groups (upward): {Team A, Department}
- Alice is not a group, so no downward expansion
- **Transitive presenter set**: {Alice, Team A, Department}
- *Not included*: Bob (sibling in Team A), Carol (in Team B), Team B

A panel directly hosted by **Team B**:

- Direct presenters: {Team B}
- Team B's inclusive groups (upward): {Department}
- Team B's inclusive members (downward): {Carol}
- **Transitive presenter set**: {Team B, Department, Carol}
- *Not included*: Alice, Bob, Team A (separate branch)

A panel directly hosted by **Department**:

- Direct presenters: {Department}
- Department has no parent groups (upward): ∅
- Department's inclusive members (downward): {Team A, Team B, Alice, Bob, Carol}
- **Transitive presenter set**: {Department, Team A, Team B, Alice, Bob, Carol}

A panel directly hosted by **Alice** and **Carol**:

- Expand Alice: {Alice, Team A, Department}
- Expand Carol: {Carol, Team B, Department}
- **Transitive presenter set**: {Alice, Carol, Team A, Team B, Department}
- *Not included*: Bob (not directly hosting, even though he's in Team A)

**Use cases** for transitive inclusion:

- **"What panels is Department involved in?"** — query all panels whose
  transitive presenter set contains Department
- **Conflict checking** — only *individuals* (leaf presenters) are
  conflict-checked for time overlaps; groups are informational
- **Credit display** — see rules below

### Transitive Closure Cache (planned)

BFS over `PresenterData.groups` (and the reverse `presenters_by_group` index)
replaces the old `PresenterToGroup` edge traversal.  Reference:
`v10-try-1/crates/schedule-core/src/data/relationship.rs`
(`RelationshipManager` / `RelationshipCache`).

The old implementation maintains four maps in a lazily-rebuilt cache:

| Map                    | Key    | Value         | Meaning                                 |
| ---------------------- | ------ | ------------- | --------------------------------------- |
| `direct_parent_groups` | member | `Vec<group>`  | Groups this member directly belongs to  |
| `direct_members`       | group  | `Vec<member>` | Members directly in this group          |
| `inclusive_groups`     | member | `Vec<group>`  | All groups (transitive) for this member |
| `inclusive_members`    | group  | `Vec<member>` | All members (transitive) in this group  |

**Algorithm** (BFS per root):

- *Inclusive members of group G*: BFS starting at G. For each direct member M,
  add M to the result set. If M is itself a group (has entries in
  `group_to_members`), push M onto the visit queue. Repeat until queue empty.
- *Inclusive groups of member M*: BFS starting at M. For each direct parent
  group G, add G to the result set and push G onto the visit queue (it may
  itself be a member of a higher group). Repeat until queue empty.

**Cycle tolerance**: the BFS uses a `visited` set so cycles (A member of B,
B member of A) terminate without infinite loops.

**Invalidation**: any change to `PresenterData.groups` (via `on_update` hook)
increments a version counter; the cache is rebuilt on next query if stale.

### Implementation Notes

`PresenterEntityType` methods (`groups_of`, `members_of`, `is_group`) provide
**direct** lookups from entity fields and the `presenters_by_group` reverse
index.  Transitive closure will be added as a `RelationshipCache` (or similar)
stored in `EntityStorage`, following the same BFS algorithm as the old
`RelationshipManager` but traversing `PresenterData.groups` Vecs instead of
edge HashMaps.

The **panel transitive presenter set** reads `PanelData.presenters` as the
direct set, then expands each via the cache's `inclusive_groups` and
`inclusive_members` maps.

### Credit Display Rules

`always_grouped` and `always_shown_in_group` are **entity-level** flags on
`Presenter` (not per-membership).  Per-membership granularity is deferred to
IDEA-039.

Spreadsheet syntax origins:

- `G:<Name=Group` → sets `always_grouped = true` on the member presenter
- `G:Name==Group` → sets `always_shown_in_group = true` on the member presenter
  - Double check this, might belong to the group presenter, see what schedule-to-html did

For a given panel, "presenting members of group G" means the direct presenters
of the panel that are inclusive members of G.

- **All members of G present** → show group name only (no individual names)
- **Partial group present, at least one presenting member has `always_grouped`**:
  - Show `"G (Member1, Member2)"` for two or more `always_grouped` members
  - Show `"Member of G"` for exactly one `always_grouped` member
  - Those `always_grouped` members are **not** listed individually alongside
    the group credit
  - Members *without* `always_grouped` are credited individually as usual
- **Partial group present, no presenting member has `always_grouped`** → show
  each presenting member individually; do not credit the group at all
- **Individual with no group** → show individual name
- **`always_shown_in_group`** — when a member is directly listed for the panel
  they are credited individually even when their group is also shown.
- **Group suppression**: if every presenting member of G has `always_shown_in_group`
  and none have `always_grouped`, the group name is **not** shown.
- Groups are never double-booked; only *individuals* (leaf presenters) are
  conflict-checked for time overlaps.

Credit display depends on **inclusive** membership when nested groups are in
play: if Alice is a transitive member of Department through Team A, she counts
as "present for Department" when checking whether all members are present.

---

## 13. UUID Collision Policy (by entity type)

| Entity    | On UUID collision        | Rationale                                            |
| --------- | ------------------------ | ---------------------------------------------------- |
| Panel     | New v7 UUID              | Duplicate Uniq ID in spreadsheet = likely data error |
| Presenter | Error or return existing | Caller should use `match_index` first                |
| EventRoom | Update existing          | Reference data; same name = same room                |
| HotelRoom | Update existing          | Reference data                                       |
| PanelType | Update existing          | Reference data; same prefix = same type              |

**Note**: This is the intended policy for spreadsheet imports where information may be
partially defined in multiple places. Further investigation during Phase 4
(import/export) may refine this based on actual usage patterns.

---

## 14. Future Phases (post META-026)

- **META-027 (Phase 3)** — CRDT integration via `rust-crdt` (op-based, `GSet`
  of operations). CRDT candidate: `automerge-rs` (document-oriented); fallback
  `crdts` (lower-level). An abstraction layer is planned so the backend can
  be swapped.

  **CRDT note**: Design is still being explored. With virtual edges (relationships
  stored as Vec fields on entities), relationship conflicts resolve via
  last-write-wins on the owning entity's field — the same as any other field.
  Referential integrity (dangling UUIDs after soft-delete) is a separate concern.
- **META-028 (Phase 4)** — File formats & import/export: XLSX round-trip,
  widget JSON v10 export (clean break from v9).
- **META-029 (Phase 5)** — CLI tools (`cosam-convert`, `cosam-modify`).
  Dynamic field access by name needed for CLI (`FieldSet::get_field(str)`
  - `DynamicField::parse_from_str`).
- **META-030 (Phase 6)** — GUI editor (`cosam-editor`). Framework deferred:
  **iced** or **GPUI**.
- **META-031 (Phase 7)** — Sync & multi-user.

---

## 15. Design Constraints & Conventions

- Rust latest stable; `cargo clippy` warnings as errors; `rustfmt` default
- `thiserror` for library errors, `anyhow` for applications
- Derive `Debug` on all public types; `Clone`, `PartialEq` where meaningful
- No `unwrap()`/`expect()` outside tests; use `?`
- One primary type per file; re-export from `mod.rs`
- Every data module has `#[cfg(test)] mod tests` block with serde round-trips
- UUID generation: v7 for new entities, v5 (deterministic) for natural-key imports
- `serde` for JSON interchange; `rkyv` under consideration for fast local snapshots
- **Logic belongs in `<Type>EntityType`**: All non-trivial implementations that
  work with entity or edge data must be methods on the relevant
  `<Type>EntityType` struct. Functions in `Schedule` and closures in
  `#[read(...)]`/`#[write(...)]` attributes are **thin adapters** — they call
  the `EntityType` method and pass the result through. No logic in adapters.
- **Use typed IDs to avoid borrow conflicts**: When an `EntityType` method needs
  to touch multiple storage maps (e.g., updating both a forward field and a
  reverse index), take `&mut EntityStorage` and typed IDs (`<Type>Id` or
  `NonNilUuid`) rather than `&mut EntityData`. This avoids borrow checker
  conflicts in computed field write closures, which already hold a mutable
  borrow of one entity while needing to access others. See `field-system.md`
  principle 9 for examples and rationale.
- Copyright header required at top of every `.rs` file:

  ```text
  // Copyright (c) 2026 Daphne Pfister
  // SPDX-License-Identifier: BSD-2-Clause
  // See LICENSE file for full license text
  ```

---

## 16. What v10-try-2 Has That cosam_sched Doesn't Yet

Consulting `v10-try-2` for these features (partially superseded design
but useful reference):

- **Query module** (`query/`) — finder/updater patterns; some ideas carry to FEATURE-009
- **`uuid_v5.rs`** — V5 UUID helpers (now absorbed into `UuidPreference::FromV5`)
- **`PresenterToGroupStorage`** with transitive closure cache — BFS algorithm
  still applies; now traverses `PresenterData.groups` Vecs + reverse index
- **`PanelToPresenter` flags** (`is_primary_presenter`, `confirmed`) — may be
  added as fields directly on `PanelData.presenters` item struct in the future

---

## 17. What v10-try-1 Has That May Be Useful

`v10-try-1/crates/schedule-core/src/` (older, largely superseded):

- `xlsx/` — XLSX import/export (umya-spreadsheet); useful reference for
  FEATURE-028 (import/export phase)
- `edit/` — older edit command design; `adjust.rs`, `command.rs`, `context.rs`,
  `find.rs` — reference for FEATURE-010
- `data/` — older entity definitions that preceded the macro system; not directly
  usable but shows what fields were needed

---

## 18. What v9 Has That May Be Useful

`v9/` (last fully functional version of schedule-core):

- `apps` -- contains the main application logic:
  - `cosam-convert` -- converts between XLSX spreadsheets and JSON v9 formats
  - `cosam-editor` -- Prototype GUI schedule editor, missing features
  - `cosam-modify` -- Prototype CLI schedule query and modification tool

---

## 19. What schedule-to-html (Perl widget) Clarifies

The Perl codebase at `desc_tbl/schedule-to-html` is the authoritative consumer
of the old spreadsheet format. Key facts:

- Consumes **Spreadsheets** only
- Panel types identified by 2-letter prefix; type properties drive display
- Room sort keys drive grid column order; `sort_key ≥ 100` = hidden
- Presenter credit display logic (groups vs individuals) lives in Perl; the
  Rust export must faithfully represent `always_grouped`, `always_shown_in_group`
  flags so the widget can apply the same rules
- `SPLIT*` rooms are filtered out; they signal page-break points in the grid
