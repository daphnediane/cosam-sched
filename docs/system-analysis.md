# cosam-sched System Analysis

Reference document for AI-assisted sessions. Describes the current state of
`cosam_sched` (active repo), source-of-truth context from related repos, and
the design decisions made so far. Update as each META-026 work item completes.

---

## 1. Repository Map

| Repo path                   | Purpose                                                                             | Trust level                                         |
| --------------------------- | ----------------------------------------------------------------------------------- | --------------------------------------------------- |
| `cosam_sched/`              | **Active workspace** — Phase 2 of META-001 in progress                              | Canonical                                           |
| `cosam-data-old/`           | v10 experiment retry: edge system + query engine, full entity definitions           | Reference (partially superseded)                    |
| `cosam-refactor/`           | v10 development sketch: schedule-core with xlsx import/export and GPUI editor shell | Reference (design ideas only; largely superseded)   |
| `cosam-preview/`            | Large Rust monorepo with display json file v9 widget integration                    | Reference (mostly out-of-date; but more functional) |
| `desc_tbl/schedule-to-html` | Perl widget / HTML schedule generator / spreadsheet only no json                    | Spreadsheet format authority                        |

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
| FEATURE-007 | Edge/relationship system                                | Stub only    |
| FEATURE-008 | Schedule container and EntityStorage                    | Near done    |
| FEATURE-009 | Query system                                            | Not started  |
| FEATURE-010 | Edit command system with undo/redo history              | Open         |

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

### 4.2 Edge-Entities (Relationships)

All relationships are stored as **first-class entities** with their own UUID
(not a separate edge store). They derive `EntityFields` and implement
`DirectedEdge`.

| Edge                   | From                 | To                  | Extra fields                              |
| ---------------------- | -------------------- | ------------------- | ----------------------------------------- |
| `PanelToPresenter`     | `Panel`              | `Presenter`         | —                                         |
| `PanelToEventRoom`     | `Panel`              | `EventRoom`         | —                                         |
| `PanelToPanelType`     | `Panel`              | `PanelType`         | —                                         |
| `EventRoomToHotelRoom` | `EventRoom`          | `HotelRoom`         | —                                         |
| `PresenterToGroup`     | `Presenter` (member) | `Presenter` (group) | `always_shown_in_group`, `always_grouped` |

**PresenterToGroup self-loop** — when `member_uuid == group_uuid`, the edge is
a *group marker*: it marks that presenter entity as a group rather than an
individual. `PresenterToGroupData::is_self_loop()` detects this.

### 4.3 Key Design Decisions

- **Edges-as-entities**: chosen so edges participate in the UUID registry, can
  be tracked by the undo system, and can carry metadata without a separate
  storage layer.
- **EdgeIndex per edge type**: each edge type has a bidirectional
  `EdgeIndex` (two `HashMap<NonNilUuid, Vec<NonNilUuid>>` for outgoing and
  incoming) stored in `EntityStorage`, kept in sync via `Schedule::add_edge`
  and `Schedule::remove_edge`.
- **Schedule is a proxy**: `Schedule` provides UUID registry coordination
  and a unified API, but entity types own their storage access patterns.
  Computed field closures access `EntityStorage` directly (via
  `TypedEdgeStorage` dispatch), not through `Schedule` convenience methods.
- **Rooms split**: `EventRoom` (logical schedule room) vs `HotelRoom` (physical
  hotel space). One event room can map to different hotel rooms at different
  times (time-partitioned), modelled by `EventRoomToHotelRoom` edges that will
  carry a `TimeRange` when FEATURE-007 is completed.

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

| Variant             | When to use                                                     |
| ------------------- | --------------------------------------------------------------- |
| `GenerateNew`       | New entity with no natural key (default, emits v7)              |
| `FromV5 { name }`   | Import from spreadsheet — deterministic from natural key string |
| `Edge { from, to }` | Edge entities — deterministic from endpoint UUIDs               |
| `Exact(uuid)`       | Restoring from serialized state                                 |

Edge builders auto-upgrade `GenerateNew` → `Edge` when both endpoints are set.

### EntityUUID / EntityKind

`EntityUUID` is a tagged enum (one variant per entity kind) returned by
`Schedule::identify()`. `EntityKind` is a plain enum used in the UUID registry.

---

## 6. Macro System (`schedule-macro`)

`#[derive(EntityFields)]` on a struct generates:

1. **`<Name>Data`** — internal storage struct with only stored fields plus
   `entity_uuid: NonNilUuid`. Computed fields appear in `Data` as their backing
   storage type, default-initialized.
2. **`<Name>EntityType`** — implements `EntityType` with `TYPE_NAME`,
   `KIND`, `type Id = <Name>Id`, `field_set()` (lazy static), and `validate()`.
3. **`<Name>Id`** — newtype ID wrapper with `TypedId` impl.
4. **`<Name>Builder`** — builder with `with_<field>()` setters,
   `build(&mut Schedule)` → `Result<Id, BuildError>` (validates, resolves UUID,
   inserts into schedule), `build_data()` → `Result<Data, ValidationError>`
   (standalone), and `apply_to()` (partial update).
5. **Per-field unit structs** — e.g. `NameField`, `UidField` — implementing
   `NamedField`, `SimpleReadableField`, `SimpleWritableField` (or `ReadableField`/
   `WritableField` for computed fields).
6. **`fields` module** — public constants for each field struct.
7. **`DirectedEdge` impl** — generated when both `#[edge_from]` and `#[edge_to]`
   are present; adds typed `from_id()` / `to_id()` accessors.

### Macro Attributes Reference

**Struct-level:**

- `#[entity_kind(Panel)]` — sets `EntityKind::Panel`, required

**Field-level:**

- `#[field(display = "…", description = "…")]` — stored field with metadata
- `#[computed_field(display = "…", description = "…")]` — user-supplied closures
- `#[alias("a", "b", …)]` — extra names in `FieldSet` name map
- `#[required]` — adds to required list; validated at `build()` time
- `#[indexable(priority = N)]` — participates in `match_index` lookups
- `#[edge_from(Entity)]` / `#[edge_to(Entity)]` — marks UUID field as edge
  endpoint; excluded from builder setters (constructor-only)

**Computed field closures:**

- `#[read(|entity: &PanelData| { … })]` — entity-only read
- `#[read(|schedule: &Schedule, entity: &PanelData| { … })]` — schedule-aware read
- `#[write(|entity: &mut PanelData, value: FieldValue| { … })]` — entity-only write
- `#[write(|schedule: &mut Schedule, entity: &mut PanelData, value: FieldValue| { … })]` — schedule-aware write

Types in closure arguments **must be fully explicit** — the macro cannot infer
them through associated type projections.

---

## 7. Field System

### FieldValue

Universal runtime field value enum:

```text
String, Integer(i64), Float(f64), Boolean, DateTime, Duration,
List(Vec<FieldValue>), Map(HashMap<String, FieldValue>),
OptionalString, OptionalInteger, OptionalFloat, OptionalBoolean,
OptionalDateTime, OptionalDuration, NonNilUuid
```

### Trait Hierarchy

```text
NamedField                    name(), display_name(), description()
├── SimpleReadableField<T>    read(&entity) → Option<FieldValue>
│   └── (blanket) ReadableField<T>
├── SimpleWritableField<T>    write(&mut entity, FieldValue) → Result
│   └── (blanket) WritableField<T>
├── IndexableField<T>         match_field(query, &entity) → Option<MatchStrength>
├── ReadableField<T>          read(&Schedule, &entity) → Option<FieldValue>  [computed]
└── WritableField<T>          write(&mut Schedule, &mut entity, FieldValue) → Result  [computed]
```

Blanket impls auto-promote `SimpleReadableField` → `ReadableField` (discards
unused schedule ref).

### FieldSet

Per-entity static registry built once in a `LazyLock`. Provides:

- `get_field(name)` — name or alias lookup → `&dyn NamedField`
- `is_required(name)`
- `match_index(query, entity)` — runs all `IndexableField`s, returns best
  `FieldMatchResult` ranked by `(match_strength, field_priority)`

Match priority levels: `EXACT_MATCH=255`, `STRONG_MATCH=200`,
`AVERAGE_MATCH=100`, `WEAK_MATCH=50`, `NO_MATCH=0`.

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

## 10. Schedule Container (FEATURE-008, in progress)

### Current Implementation

```text
Schedule
├── entities: EntityStorage — per-type HashMap<NonNilUuid, Data>
│   ├── panels: HashMap<NonNilUuid, PanelData>
│   ├── presenters: HashMap<NonNilUuid, PresenterData>
│   ├── event_rooms: HashMap<NonNilUuid, EventRoomData>
│   ├── hotel_rooms: HashMap<NonNilUuid, HotelRoomData>
│   ├── panel_types: HashMap<NonNilUuid, PanelTypeData>
│   ├── panel_to_presenters: HashMap + EdgeIndex
│   ├── panel_to_event_rooms: HashMap + EdgeIndex
│   ├── panel_to_panel_types: HashMap + EdgeIndex
│   ├── event_room_to_hotel_rooms: HashMap + EdgeIndex
│   └── presenter_to_groups: HashMap + EdgeIndex
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
}
```

The `type Id` associated type links each entity type to its typed ID wrapper,
enabling generic methods like `add_entity` to return the correct ID type.

### Builder → Schedule Integration

`Builder::build(&mut Schedule)` validates required fields, resolves the UUID
(via `UuidPreference`), constructs the data struct, and inserts it into the
schedule in one step. Returns `Result<TypedId, BuildError>` where `BuildError`
combines `ValidationError` and `InsertError`.

`Builder::build_data()` produces the data struct without inserting (useful for
tests or deferred insertion).

### EdgeIndex and TypedEdgeStorage

`EdgeIndex` holds two `HashMap<NonNilUuid, Vec<NonNilUuid>>` maps (outgoing
and incoming). Each edge type has one `EdgeIndex` stored in `EntityStorage`.

`TypedEdgeStorage` trait (analogous to `TypedStorage` for node entities) maps
each edge entity type to its `EdgeIndex` field and typed `HashMap`, enabling
compile-time dispatch:

```rust
pub trait TypedEdgeStorage: EntityType {
    fn edge_index(storage: &EntityStorage) -> &EdgeIndex;
    fn edge_index_mut(storage: &mut EntityStorage) -> &mut EdgeIndex;
    fn typed_map(storage: &EntityStorage) -> &HashMap<NonNilUuid, Self::Data>;
}
```

### Edge Convenience Methods

Each edge `EntityType` has static convenience query methods that encapsulate
the `TypedEdgeStorage` + `TypedStorage` lookups:

| Edge EntityType                  | Methods                                           |
| -------------------------------- | ------------------------------------------------- |
| `PanelToPresenterEntityType`     | `presenters_of(storage, uuid)`, `panels_of`       |
| `PanelToEventRoomEntityType`     | `event_room_of(storage, uuid)`, `panels_in`       |
| `PanelToPanelTypeEntityType`     | `panel_type_of(storage, uuid)`, `panels_of_type`  |
| `EventRoomToHotelRoomEntityType` | `hotel_rooms_of(storage, uuid)`, `event_rooms_in` |
| `PresenterToGroupEntityType`     | `groups_of`, `members_of`, `is_group`             |

These take `&EntityStorage` (not `&Schedule`), reinforcing the principle that
entity types own their storage access.

`Schedule` has thin wrappers (e.g. `get_panel_presenters`) that delegate to
these methods.

### Edge-Aware Computed Fields

Panel computed fields (`presenters`, `event_room`, `panel_type`) use the
schedule-aware read closure signature and call edge EntityType convenience
methods directly on `&schedule.entities`:

```rust
#[read(|schedule: &Schedule, entity: &PanelData| {
    let ids = PanelToPresenterEntityType::presenters_of(&schedule.entities, entity.uuid());
    // ... convert to FieldValue
})]
pub presenters: Vec<PresenterId>,
```

This ensures computed fields work at the `EntityStorage` level without
circular dependency on `Schedule` convenience methods.

### Membership Mutation Helpers

`Schedule` provides convenience methods for managing `PresenterToGroup` edges:

| Method                              | Effect                                              |
| ----------------------------------- | --------------------------------------------------- |
| `mark_presenter_group(id)`          | Add self-loop group marker                          |
| `unmark_presenter_group(id)`        | Remove self-loop group marker                       |
| `add_member(member, group)`         | Add membership edge (no flag changes if exists)     |
| `add_grouped_member(member, group)` | Add/update edge with `always_grouped = true`        |
| `add_shown_member(member, group)`   | Add/update edge with `always_shown_in_group = true` |
| `remove_member(member, group)`      | Remove membership edge                              |

### Presenter Tag-String Lookup

`PresenterEntityType::lookup_tagged(schedule: &mut Schedule, input: &str) -> Result<PresenterId, LookupError>`
is the implementation in `entity/presenter.rs`.  Also exposed as the thin
delegate `Schedule::lookup_tagged_presenter(input)`.  Handles UUID references,
tagged credit strings, and bare name lookups.  Documented fully in FEATURE-009.

`PresenterEntityType::find_or_create_by_name(schedule, name, rank)` is a public
helper for callers that already know the name and rank directly.

`LookupError`: `Empty`, `UuidNotFound`, `InvalidUuid`, `NameNotFound`,
`UnknownTag`, `OtherSentinel`.

### Not Yet Implemented

- **Edge uniqueness** policies (`Reject`, `Replace`, `Allow`)
- **`PresenterToGroupStorage`** with transitive closure cache
- **Entity name lookup** (`get_entity_names`) — stub exists

---

## 11. Edit System (FEATURE-010, not yet implemented)

Planned `EditCommand` enum wrapping reversible operations:
`UpdateField`, `AddEntity`, `RemoveEntity`, `AddEdge`, `RemoveEdge`,
`MovePanel`, `BatchEdit`.

`EditHistory` — stack-based undo/redo with configurable max depth.

`EditContext` = `Schedule` + `EditHistory`. All public mutations go through it.

This is also the **CRDT integration point** (Phase 3 / META-027): applied
commands can emit CRDT operations for peer broadcast.

---

## 12. Presenter Group Semantics

Groups are `Presenter` entities with `is_group = true`. Membership is encoded
in `PresenterToGroup` edges.

| Edge type  | `member_uuid` | `group_uuid` | Meaning                 |
| ---------- | ------------- | ------------ | ----------------------- |
| Self-loop  | X             | X            | Marks X as a group      |
| Membership | member        | group        | member belongs to group |

`always_grouped` on the edge — this member always appears under the group name,
never individually.  
`always_shown_in_group` on the edge — the group is shown even when not all
members are present.

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

A panel's **direct presenters** are the `Presenter` entities linked via
`PanelToPresenter` edges. The **transitive presenter set** for a panel is the
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

Reference: `cosam-refactor/crates/schedule-core/src/data/relationship.rs`
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

**Invalidation**: any edge add/remove increments a version counter; the cache
is rebuilt on next query if stale.

### Adapting to UUID-Based Edges

The current `PresenterToGroupEntityType` convenience methods (`groups_of`,
`members_of`, `is_group`) provide **direct** lookups only. Transitive closure
will be added as a `RelationshipCache` (or similar) stored alongside the
`EdgeIndex` in `EntityStorage`, using UUIDs instead of name strings. The
algorithm and invalidation strategy will follow the same pattern as the old
`RelationshipManager`.

The **panel transitive presenter set** computation will use the cache's
`inclusive_groups` and `inclusive_members` maps, expanding each direct presenter
both upward and downward as described above.

### Credit Display Rules

Both `always_grouped` and `always_shown_in_group` are flags on the
**`PresenterToGroup` edge**, not on the presenter entity. A presenter can have
different flag values in different group memberships (e.g. `always_grouped`
with respect to Group A but not Group B).

Spreadsheet syntax origins:

- `G:<Name=Group` → sets `always_grouped = true` on the member→group edge
  (this *member* always appears credited under the group, not individually)
- `G:Name==Group` → sets `always_shown_in_group = true` on the member→group edge
  (this *group* should always be shown when this member is present, even if
  others in the group are absent)

For a given panel, "presenting members of group G" means the direct presenters
of the panel that are inclusive members of G.

- **All members of G present** → show group name only (no individual names)
- **Partial group present, at least one presenting member has `always_grouped`
  on their G-membership edge**:
  - Show `"G (Member1, Member2)"` for two or more `always_grouped` members
  - Show `"Member of G"` for exactly one `always_grouped` member
  - Those `always_grouped` members are **not** listed individually alongside
    the group credit
  - Members *without* `always_grouped` on this edge are credited individually
    as usual
- **Partial group present, no presenting member has `always_grouped`** → show
  each presenting member individually; do not credit the group at all
- **Individual with no group** → show individual name
- **`always_shown_in_group` on the edge** — when a member is *directly listed*
  for the panel (i.e. has their own `PanelToPresenter` edge), they are credited
  individually even when their group is also shown. `always_shown_in_group`
  does not apply if the member appears only transitively.
- **Group suppression**: if every presenting member of G has `always_shown_in_group`
  on their G-membership edge and none have `always_grouped`, the group name is
  **not** shown — those members are credited individually. The group acts purely
  as an organisational/filtering mechanism with no effect on visible credits.
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

  **CRDT note**: Design is still being explored. Edges are only valid if the
  entities they point to are valid. Last-write-wins on entity fields is a
  reasonable default, but edge conflict resolution may need special handling
  to ensure endpoint integrity.
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
- UUID generation: v7 for new entities, v5 (deterministic) for imports/edges
- `serde` for JSON interchange; `rkyv` under consideration for fast local snapshots
- Copyright header required at top of every `.rs` file:

  ```text
  // Copyright (c) 2026 Daphne Pfister
  // SPDX-License-Identifier: BSD-2-Clause
  // See LICENSE file for full license text
  ```

---

## 16. What cosam-data-old Has That cosam_sched Doesn't Yet

Consulting `cosam-data-old` for these features (partially superseded design
but useful reference):

- **Full edge query engine** (`edge_entity_query.rs`, ~24 KB) — complex query
  patterns over edge-entities; some patterns will carry over to FEATURE-008/009
- **Query module** (`query/`) — finder/updater patterns
- **`uuid_v5.rs`** — V5 UUID helpers (now absorbed into `UuidPreference::FromV5`)
- **`PresenterToGroupStorage`** with transitive closure cache — will be
  incorporated into FEATURE-008's specialized edge storage
- **Full `PanelToPresenter` edge** with `is_primary_presenter`, `confirmed`
  flags — the current edge is a stub; these fields may be added

---

## 17. What cosam-refactor Has That May Be Useful

`cosam-refactor/crates/schedule-core/src/` (older, largely superseded):

- `xlsx/` — XLSX import/export (umya-spreadsheet); useful reference for
  FEATURE-028 (import/export phase)
- `edit/` — older edit command design; `adjust.rs`, `command.rs`, `context.rs`,
  `find.rs` — reference for FEATURE-010
- `data/` — older entity definitions that preceded the macro system; not directly
  usable but shows what fields were needed

---

## 18. What schedule-to-html (Perl widget) Clarifies

The Perl codebase at `desc_tbl/schedule-to-html` is the authoritative consumer
of the schedule JSON widget export. Key facts:

- Consumes **v9 JSON format** (the widget in `cosam_sched/widget/` is v9-based)
- Panel types identified by 2-letter prefix; type properties drive display
- Room sort keys drive grid column order; `sort_key ≥ 100` = hidden
- Presenter credit display logic (groups vs individuals) lives in Perl; the
  Rust export must faithfully represent `always_grouped`, `always_shown_in_group`
  flags so the widget can apply the same rules
- `SPLIT*` rooms are filtered out; they signal page-break points in the grid

The new export will target **v10 JSON** (clean break from v9); v9 compatibility
is handled by `cosam-convert` if needed.
