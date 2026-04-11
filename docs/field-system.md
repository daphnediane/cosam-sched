# Entity Field System

Complete reference for the proc-macro-based field system used in `cosam_sched`. This document serves both human developers and AI assistants working on the codebase.

## Overview

The field system provides type-safe entity definitions with automatic code generation via the `#[derive(EntityFields)]` macro from `schedule-macro`. It supports:

- **Stored fields**: Direct data storage in the entity struct
- **Computed fields**: Dynamic values resolved via closures (can access schedule/edges)
- **Edge entities**: First-class relationships with UUID endpoints
- **Indexable fields**: Fast lookup by field value with match strength
- **Validation**: Required field checking and custom validation

---

## Architecture

### Crate Layout

```text
crates/
├── schedule-macro/          # #[derive(EntityFields)] proc-macro
│   └── src/
│       └── lib.rs           # Entry point and macro implementation
│
└── schedule-data/             # Data model and runtime support
    └── src/
        ├── entity/            # Entity definitions
        │   ├── panel.rs
        │   ├── presenter.rs
        │   ├── event_room.rs
        │   ├── panel_to_presenter.rs  # Edge entity
        │   └── ...
        ├── field/             # Field system traits and types
        │   ├── traits.rs      # Field trait hierarchy
        │   ├── types.rs       # FieldValue enum
        │   └── field_set.rs   # FieldSet registry
        └── schedule/          # Schedule container
            ├── storage.rs     # EntityStorage, EntityStore
            └── edge_index.rs  # EdgeIndex for bidirectional lookups
```

### Core Types

| Type            | Purpose                              | Location              |
| --------------- | ------------------------------------ | --------------------- |
| `FieldValue`    | Universal runtime field value enum   | `field/types.rs`      |
| `FieldSet<T>`   | Per-entity static field registry     | `field/field_set.rs`  |
| `EntityStorage` | Per-type HashMap storage + EdgeIndex | `schedule/storage.rs` |
| `Schedule`      | UUID registry + storage proxy        | `schedule/mod.rs`     |
| `NonNilUuid`    | Non-nil UUID wrapper                 | `entity/mod.rs`       |

---

## Field Trait Hierarchy

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

**Blanket implementations** automatically promote `Simple*Field` → `*Field` by discarding the unused schedule reference.

---

## Entity Definition

### Basic Entity (Stored Fields Only)

```rust
use cosam_sched::entity::EntityFields;
use cosam_sched::field::{field, indexable, required};
use cosam_sched::entity::NonNilUuid;

#[derive(EntityFields, Debug, Clone)]
#[entity_kind(EventRoom)]
pub struct EventRoom {
    pub entity_uuid: NonNilUuid,

    #[field(display = "Room Name", description = "Short room identifier")]
    #[alias("short", "room_name")]
    #[required]
    #[indexable(priority = 220)]
    pub room_name: String,

    #[field(display = "Long Name", description = "Human-readable room name")]
    #[alias("long_name")]
    #[indexable(priority = 210)]
    pub long_name: Option<String>,

    #[field(display = "Sort Key", description = "Display order (≥100 = hidden)")]
    pub sort_key: i64,
}
```

### Entity with Computed Fields

```rust
use cosam_sched::entity::EntityFields;
use cosam_sched::field::{computed_field, read, write};
use cosam_sched::schedule::Schedule;

#[derive(EntityFields, Debug, Clone)]
#[entity_kind(Panel)]
pub struct Panel {
    pub entity_uuid: NonNilUuid,

    // Stored field
    #[field(display = "Panel Name", description = "Panel title")]
    #[required]
    pub name: String,

    // Computed field - reads from edge relationships
    #[computed_field(display = "Presenters", description = "Panel presenters")]
    #[read(|schedule: &Schedule, entity: &PanelData| {
        let ids = PanelToPresenterEntityType::presenters_of(&schedule.entities, entity.uuid());
        Some(FieldValue::from(ids))
    })]
    pub presenters: Vec<PresenterId>,
}
```

### Edge Entity (Relationship)

```rust
use cosam_sched::entity::EntityFields;
use cosam_sched::field::{edge_from, edge_to, field};

#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PanelToPresenter)]
pub struct PanelToPresenter {
    pub entity_uuid: NonNilUuid,

    // Edge endpoints — left/right sides, immutable after construction, excluded from builder setters
    #[edge_from(Panel)]       // left side → generates left_id(), left_uuid()
    pub panel_uuid: NonNilUuid,

    #[edge_to(Presenter)]     // right side → generates right_id(), right_uuid()
    pub presenter_uuid: NonNilUuid,

    // Optional edge metadata
    #[field(display = "Is Primary", description = "Primary presenter for this panel")]
    pub is_primary: bool,
}
```

---

## Macro Attributes Reference

### Struct-Level

| Attribute               | Purpose                            | Example                 |
| ----------------------- | ---------------------------------- | ----------------------- |
| `#[entity_kind(Panel)]` | Sets `EntityKind::Panel`, required | `#[entity_kind(Panel)]` |

### Field-Level (Stored Fields)

| Attribute                                        | Purpose                               | Example                                                             |
| ------------------------------------------------ | ------------------------------------- | ------------------------------------------------------------------- |
| `#[field(display = "...", description = "...")]` | Field metadata                        | `#[field(display = "Room Name", description = "Short identifier")]` |
| `#[alias("a", "b")]`                             | Extra names in FieldSet lookup        | `#[alias("short", "room_name")]`                                    |
| `#[required]`                                    | Adds to required list for validation  | `#[required]`                                                       |
| `#[indexable(priority = N)]`                     | Participates in `match_index` lookups | `#[indexable(priority = 220)]`                                      |

### Field-Level (Edge Endpoints)

| Attribute                               | Purpose                                                        | Example                                         |
| --------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------- |
| `#[edge_from(Entity)]`                  | Marks UUID field as left-side endpoint, excluded from builder  | `#[edge_from(Panel)]`                           |
| `#[edge_to(Entity)]`                    | Marks UUID field as right-side endpoint, excluded from builder | `#[edge_to(Presenter)]`                         |
| `#[edge_from(Entity, accessor = name)]` | Same, but overrides generated accessor method name             | `#[edge_from(Presenter, accessor = member_id)]` |
| `#[edge_to(Entity, accessor = name)]`   | Same, but overrides generated accessor method name             | `#[edge_to(Presenter, accessor = group_id)]`    |

Both `#[edge_from]` and `#[edge_to]` together generate a `DirectedEdge` implementation with
`left_id()`, `left_uuid()`, `right_id()`, `right_uuid()` methods.

> **Why `left`/`right` instead of `from`/`to`?**  Edge relationships are often
> bidirectional in meaning — "Panel hosted by Presenter" is equivalent to
> "Presenter hosts Panel".  `from`/`to` implies false directionality and conflicts
> with Rust's `From`/`Into` conversion naming conventions.  `left`/`right` is
> positionally neutral.

### Field-Level (Computed Fields)

| Attribute                                                 | Purpose                        | Example                                                           |
| --------------------------------------------------------- | ------------------------------ | ----------------------------------------------------------------- |
| `#[computed_field(display = "...", description = "...")]` | Marks as computed (no storage) | `#[computed_field(display = "Presenters")]`                       |
| `#[read(\|...\| { ... })]`                                | Read closure                   | `#[read(\|entity: &PanelData\| { ... })]`                         |
| `#[write(\|...\| { ... })]`                               | Write closure                  | `#[write(\|entity: &mut PanelData, value: FieldValue\| { ... })]` |

**CRITICAL**: Closure parameters **must** have explicit type annotations. The macro cannot infer types through associated type projections.

#### Read Closure Signatures

```rust
// Simple read (entity only)
#[read(|entity: &PanelData| { ... })]

// Schedule-aware read (for edge access)
#[read(|schedule: &Schedule, entity: &PanelData| { ... })]
```

#### Write Closure Signatures

```rust
// Simple write (entity only)
#[write(|entity: &mut PanelData, value: FieldValue| { ... })]

// Schedule-aware write (for edge mutation)
#[write(|schedule: &mut Schedule, entity: &mut PanelData, value: FieldValue| { ... })]
```

---

## Generated Code

For each entity, the macro generates:

1. **`<Name>Data`** — Internal storage struct with only stored fields plus `entity_uuid: NonNilUuid`
2. **`<Name>EntityType`** — Implements `EntityType` with:
   - `TYPE_NAME` (e.g., `"panel"`)
   - `KIND` (e.g., `EntityKind::Panel`)
   - `type Id = <Name>Id`
   - `field_set()` — lazy static `FieldSet<Self>`
   - `validate()` — checks required fields
3. **`<Name>Id`** — Newtype wrapper around `NonNilUuid` implementing `TypedId`
4. **`<Name>Builder`** — Builder pattern with:
   - `with_<field>()` setters for stored fields
   - `build(&mut Schedule)` → `Result<Id, BuildError>`
   - `build_data()` → `Result<Data, ValidationError>` (standalone)
   - `apply_to(&mut Schedule, id)` — partial update
5. **Per-field unit structs** — e.g., `NameField`, `UidField` implementing field traits
6. **`fields` module** — Public constants for each field struct
7. **`DirectedEdge` impl** — When both `#[edge_from]` and `#[edge_to]` are present;
   generates `left_id()`, `left_uuid()`, `right_id()`, `right_uuid()`, `is_self_loop()`

---

## FieldValue Type

Universal runtime field value enum:

```rust
pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(DateTime<Utc>),
    Duration(Duration),
    List(Vec<FieldValue>),
    Map(HashMap<String, FieldValue>),
    OptionalString(Option<String>),
    OptionalInteger(Option<i64>),
    OptionalFloat(Option<f64>),
    OptionalBoolean(Option<bool>),
    OptionalDateTime(Option<DateTime<Utc>>),
    OptionalDuration(Option<Duration>),
    NonNilUuid(NonNilUuid),
}
```

### Conversions

```rust
// From primitive types
let fv: FieldValue = "hello".into();
let fv: FieldValue = 42i64.into();
let fv: FieldValue = true.into();

// From typed IDs
let presenter_id: PresenterId = ...;
let fv = FieldValue::from(presenter_id.non_nil_uuid());

// From collections
let ids: Vec<PresenterId> = vec![...];
let fv = FieldValue::from(ids);  // Converts to List of NonNilUuids
```

---

## Schedule Storage Pattern

### Storage Architecture

```text
Schedule
├── entities: EntityStorage          // Per-type storage + EdgeIndex
│   ├── panels: HashMap<NonNilUuid, PanelData>
│   ├── presenters: HashMap<NonNilUuid, PresenterData>
│   └── ... (one per entity type)
├── edge_indices                      // Bidirectional edge lookups
│   ├── panel_to_presenter: EdgeIndex
│   └── ... (one per edge type)
├── uuid_registry: HashMap<NonNilUuid, EntityKind>
└── metadata: ScheduleMetadata
```

### EntityStore Trait

```rust
pub trait EntityStore<T: EntityType> {
    fn get_entity(&self, uuid: NonNilUuid) -> Option<&T::Data>;
    fn get_entity_mut(&mut self, uuid: NonNilUuid) -> Option<&mut T::Data>;
    fn insert_entity(&mut self, uuid: NonNilUuid, data: T::Data) -> Result<(), InsertError>;
    fn remove_entity(&mut self, uuid: NonNilUuid) -> Option<T::Data>;
    fn contains_entity(&self, uuid: NonNilUuid) -> bool;
}
```

**Implementation chain:**

- `EntityStorage` implements `EntityStore<T>` via blanket impl for all `T: TypedStorage`
- `Schedule` implements `EntityStore<T>`, adding UUID registry management

---

## Edge System

### Edge as First-Class Entity

Edges are stored as entities with their own UUID (not a separate edge store). This enables:

- Participation in UUID registry and undo system
- Metadata storage on relationships
- Unified API for all entities

### EdgeIndex

Each edge type has a bidirectional `EdgeIndex`:

```rust
pub struct EdgeIndex {
    outgoing: HashMap<NonNilUuid, Vec<NonNilUuid>>,  // from -> [to, to, ...]
    incoming: HashMap<NonNilUuid, Vec<NonNilUuid>>,  // to -> [from, from, ...]
}
```

Kept in sync via `Schedule::add_edge()` and `Schedule::remove_edge()`.

### Edge UUID Derivation

Edge UUIDs are deterministic from endpoints (V5 UUID):

```rust
// When building, auto-upgrade from GenerateNew to Edge preference
let edge = PanelToPresenterBuilder::new()
    .with_panel_uuid(panel_uuid)
    .with_presenter_uuid(presenter_uuid)
    .build(schedule)?;  // UUID derived from (panel_uuid, presenter_uuid)
```

### TypedEdgeStorage

Compile-time dispatch to edge-specific storage:

```rust
pub trait TypedEdgeStorage: EntityType {
    fn edge_index(storage: &EntityStorage) -> &EdgeIndex;
    fn edge_index_mut(storage: &mut EntityStorage) -> &mut EdgeIndex;
    fn typed_map(storage: &EntityStorage) -> &HashMap<NonNilUuid, Self::Data>;
}
```

### Edge Convenience Methods

Each edge `EntityType` provides static query methods:

```rust
// PanelToPresenterEntityType
pub fn presenters_of(storage: &EntityStorage, panel_uuid: NonNilUuid) -> Vec<PresenterId>;
pub fn panels_of(storage: &EntityStorage, presenter_uuid: NonNilUuid) -> Vec<PanelId>;

// PanelToEventRoomEntityType  
pub fn event_room_of(storage: &EntityStorage, panel_uuid: NonNilUuid) -> Option<EventRoomId>;
pub fn panels_in(storage: &EntityStorage, room_uuid: NonNilUuid) -> Vec<PanelId>;
```

These take `&EntityStorage` (not `&Schedule`), reinforcing that entity types own their storage access.

---

## Indexing and Lookup

### Match Strength Levels

```rust
pub const EXACT_MATCH: u8 = 255;      // Case-insensitive exact match
pub const STRONG_MATCH: u8 = 200;     // Prefix match
pub const AVERAGE_MATCH: u8 = 100;    // Substring match
pub const WEAK_MATCH: u8 = 50;        // Weak substring
pub const NO_MATCH: u8 = 0;
```

### FieldSet.match_index()

```rust
impl<T: EntityType> FieldSet<T> {
    pub fn match_index(
        &self,
        query: &str,
        uuid: NonNilUuid,
        entity: &T::Data,
    ) -> Option<FieldMatchResult> {
        // Iterate all IndexableField<T>, return best match by (strength, priority)
    }
}
```

### EntityStorage.get_by_index()

```rust
impl EntityStorage {
    pub fn get_by_index<T: TypedStorage>(
        &self,
        query: &str,
    ) -> Vec<&T::Data> {
        // Returns all entities with STRONG_MATCH or better
    }
}
```

---

## UUID System

### TypedId Wrappers

```rust
pub struct PanelId(NonNilUuid);
pub struct PresenterId(NonNilUuid);
// etc.
```

Each implements:

- `TypedId` trait: `.non_nil_uuid()`, `.uuid()`, `.kind()`, `from_uuid()`, `try_from_raw_uuid()`
- Kebab-prefixed `Display`: `"panel-<uuid>"`, `"presenter-<uuid>"`
- Serde as plain UUID string (no prefix in JSON)
- Per-type UUID namespace (`cosam.<snake_case_name>` hashed with v5 DNS)

### UuidPreference

Controls UUID assignment in builders:

| Variant             | Use Case                                                 |
| ------------------- | -------------------------------------------------------- |
| `GenerateNew`       | New entity with no natural key (default, emits v7)       |
| `FromV5 { name }`   | Import from spreadsheet — deterministic from natural key |
| `Edge { from, to }` | Edge entities — deterministic from endpoint UUIDs        |
| `Exact(uuid)`       | Restoring from serialized state                          |

Edge builders auto-upgrade `GenerateNew` → `Edge` when both endpoints are set.

---

## Common Patterns

### Adding a New Entity Type

1. Create `entity/my_entity.rs` with copyright header
2. Add `use crate::EntityFields;`
3. Define `#[derive(EntityFields, Debug, Clone)]` struct with field annotations
4. Add `pub mod my_entity;` to `entity/mod.rs`
5. Add explicit re-export: `pub use my_entity::MyEntity;` (no glob imports)
6. Run `cargo check`

### Adding a Field to an Entity

1. Add Rust field with `#[field(display = "...", description = "...")]`
2. Optionally add `#[alias(...)]`, `#[required]`, `#[indexable(priority = N)]`
3. For computed types: use `#[computed_field(...)]` with `#[read(...)]`/`#[write(...)]`
4. Run `cargo check`

### Creating an Edge Relationship

1. Define edge entity with `#[edge_from(Source)]` and `#[edge_to(Target)]`
2. Access via computed fields using `TypedEdgeStorage` methods:

   ```rust
   PanelToPresenterEntityType::presenters_of(&schedule.entities, panel_uuid)
   ```

3. Modify via schedule-aware write closures

### Looking Up Entities by Name

```rust
// Via EntityType
let presenters: Vec<&PresenterData> = storage.get_by_index::<PresenterEntityType>("Alice");

// Via Schedule helper
let id = schedule.lookup_tagged_presenter("G:Alice")?;
```

---

## Design Principles

1. **`<Type>EntityType` owns the logic** — All non-trivial implementations that operate on entity or edge data belong as methods on the corresponding `<Type>EntityType` struct (e.g. `PanelToPresenterEntityType::presenters_of`, `PresenterEntityType::lookup_tagged`). This keeps logic testable, co-located with the data it operates on, and independent of the `Schedule` API surface.
2. **`Schedule` methods are thin adapters** — Convenience methods on `Schedule` (e.g. `get_panel_presenters`, `lookup_tagged_presenter`) must not contain logic. They call the `EntityType` implementation and return the result. A `Schedule` method that does more than one non-trivial call is a smell.
3. **Computed field closures are thin adapters** — `#[read(...)]` and `#[write(...)]` closures must not contain business logic. They call the appropriate `EntityType` method and convert the return value to/from `FieldValue`. Logic embedded directly in a closure cannot be unit-tested without a full `Schedule`.
4. **Schedule is a proxy, not an owner** — Entity types own their storage; `Schedule` provides UUID registry and unified API
5. **EntityStorage is the authority** — Computed field closures access `EntityStorage` directly via `TypedStorage`/`TypedEdgeStorage` dispatch
6. **Edges as entities** — Relationships are first-class entities with UUIDs, not a separate storage layer
7. **Explicit types in closures** — Macro-generated code requires full type annotations in computed field closures
8. **No unwrap/expect in production** — Use `?` and proper error handling

### Adapter Pattern Example

```rust
// ✅ CORRECT — logic in EntityType, closure is a thin adapter
impl PanelToPresenterEntityType {
    pub fn presenters_of(storage: &EntityStorage, panel_uuid: NonNilUuid) -> Vec<PresenterId> {
        // ... real implementation here
    }
}

// In Panel entity definition:
#[read(|schedule: &Schedule, entity: &PanelData| {
    let ids = PanelToPresenterEntityType::presenters_of(&schedule.entities, entity.uuid());
    Some(FieldValue::from(ids))  // thin: call + convert only
})]
pub presenters: Vec<PresenterId>,

// In Schedule:
pub fn get_panel_presenters(&self, panel_id: PanelId) -> Vec<PresenterId> {
    PanelToPresenterEntityType::presenters_of(&self.entities, panel_id.non_nil_uuid())
}

// ❌ WRONG — logic embedded in closure or Schedule method
#[read(|schedule: &Schedule, entity: &PanelData| {
    // Don't implement traversal logic here — put it in EntityType
    let index = PanelToPresenterEntityType::edge_index(&schedule.entities);
    let uuids = index.outgoing.get(&entity.uuid()).cloned().unwrap_or_default();
    // ... more logic ...
})]
```

---

## See Also

- `system-analysis.md` — Overall system architecture and design decisions
- `spreadsheet-format.md` — XLSX format reference for import/export
- `json-v10-full.md` — JSON v10 format specification
- Source files in `crates/schedule-data/src/entity/` for working examples
