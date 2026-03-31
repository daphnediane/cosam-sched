# Entity Field System Architecture

This project uses a proc-macro-based field system for type-safe entity definitions.
All entity structs live in `crates/schedule-data/src/entity/` and derive their field
trait implementations via `#[derive(EntityFields)]` from the `schedule-macro` crate.

## Crate Layout

- **`schedule-macro`** — proc-macro crate, exports `EntityFields` derive
- **`schedule-data`** — data model crate, depends on `schedule-macro`
  - `entity/mod.rs` — `EntityType` trait, `EntityId` (`u64`), `EntityState`
  - `entity/{panel,presenter,room,panel_type,edge}.rs` — entity structs
  - `field/traits.rs` — field trait hierarchy
  - `field/field_set.rs` — `FieldSet<T>` (field registry per entity type)
  - `field/types.rs` — `FieldValue` enum (String, Integer, Boolean, etc.)
  - `field/update_logic.rs` — `FieldUpdater` trait (stubbed, future work)
  - `schedule/storage.rs` — `EntityStorage` with `get_by_index`

## Trait Hierarchy

```text
NamedField                    (name, display_name, description)
├── SimpleReadableField<T>    (read without schedule)
├── SimpleWritableField<T>    (write without schedule)
├── SimpleCheckedField<T>     (validate without schedule)
├── IndexableField<T>         (match_field for lookup, priority)
├── ReadableField<T>          (read with &Schedule)
├── WritableField<T>          (write with &Schedule)
└── CheckedField<T>           (validate with &Schedule)

Blanket impls: SimpleReadable → Readable, SimpleWritable → Writable,
               SimpleChecked → Checked  (ignore the schedule parameter)

Combo traits:  SimpleField<T> = SimpleReadable + SimpleWritable
               Field<T>       = Readable + Writable
```

`NamedField` is **not** generic over `T`. All other field traits are generic
over `T: EntityType`.

## EntityType Trait

```rust
pub trait EntityType: 'static + Send + Sync + Sized {
    type Data: Clone + Send + Sync + fmt::Debug;  // usually Self
    const TYPE_NAME: &'static str;                 // e.g. "room"
    fn field_set() -> &'static FieldSet<Self>;
    fn validate(data: &Self::Data) -> Result<(), ValidationError>;
}
```

The macro generates `EntityType` with `type Data = Self` and
`TYPE_NAME` = lowercase struct name. `field_set()` returns a
`LazyLock<FieldSet<Self>>` containing all fields, the name/alias map,
required field list, and indexable field list.

## Macro Attributes Reference

### On the struct

```rust
#[derive(EntityFields, Debug, Clone)]
pub struct Room { ... }
```

### On direct fields

```rust
#[field(display = "Room Name", description = "Short room name")]
#[alias("short", "room_name")]       // extra lookup names
#[required]                           // validation: must be non-empty/non-None
#[indexable(priority = 180)]          // participate in get_by_index lookups
pub short_name: String,
```

Supported direct field types: `String`, `i64`, `i32`, `u64`, `u32`, `bool`,
`EntityId` (alias for `u64`), and `Option<T>` of each, plus
`HashMap<String, FieldValue>`.

### On computed fields

```rust
#[computed_field(display = "Edge Type", description = "Type of relationship")]
#[alias("type", "edge_type")]
#[read(|entity: &Edge| { ... -> Option<FieldValue> })]
#[write(|entity: &mut Edge, value: FieldValue| { ... -> Result<(), FieldError> })]
pub edge_type: EdgeType,
```

**Important**: Closure parameters MUST have explicit type annotations
(e.g. `entity: &Edge`, not just `entity`). The macro cannot infer types
through associated type projections.

For closures that need schedule access, add `schedule` as the first parameter:

```rust
#[read(|schedule: &Schedule, entity: &Panel| { ... })]
#[write(|schedule: &Schedule, entity: &mut Panel, value: FieldValue| { ... })]
```

The macro detects schedule-dependent closures by checking if the token stream
contains `schedule`.

### Field naming

- `#[field_name("custom_name")]` — override the internal field name
  (default: Rust field name as string)
- `#[field_const("CUSTOM_CONST")]` — override the generated constant name

## Generated Code

For each `#[field]` field, the macro generates:

1. A unit struct (e.g. `pub struct ShortNameField;`)
2. `impl NamedField for ShortNameField { ... }`
3. `impl SimpleReadableField<Room> for ShortNameField { ... }`
4. `impl SimpleWritableField<Room> for ShortNameField { ... }`
5. A `pub static SHORT_NAME_FIELD: ShortNameField = ShortNameField;` constant

For `#[computed_field]`, steps 3-4 use the user-provided closures and may
implement `ReadableField`/`WritableField` (with schedule) instead of the
Simple variants.

A `pub mod fields { ... }` submodule is generated containing all field structs
and constants.

The macro also generates:

- `impl EntityType for Room { ... }` with `type Data = Room`
- `fn field_set() -> &'static FieldSet<Room>` using `LazyLock`
- `fn validate(data: &Room) -> Result<(), ValidationError>` checking required fields

## FieldSet and Indexing

`FieldSet<T>` holds:

- `fields` — all field references
- `name_map` — `(name_or_alias, field_ref)` pairs for `get_field()`
- `required_fields` — names checked during validation
- `indexable_fields` — fields that participate in `match_index()`

`match_index(query, entity_id, entity)` iterates indexable fields, calls
`IndexableField::match_field()`, and returns the single best `FieldMatchResult`
ranked by `(strength, priority)`.

`EntityStorage::get_by_index<T>(query)` iterates all entities of type T,
calls `match_index` on each, collects entities at the best `MatchStrength`,
and returns `Vec<&T::Data>`.

## Edge System

Edges represent relationships between entities. `EdgeType` encodes the
relationship kind (e.g. `PanelToPresenter`, `PresenterToGroup`).
Edge `from_uid` and `to_uid` are `EntityId` (u64). Previously these were
`Vec<String>` fields on entities; now they're separate `Edge` entities.

## How to Add a New Entity

1. Create `crates/schedule-data/src/entity/my_entity.rs`
2. Add copyright header
3. `use crate::EntityFields;`
4. Define struct with `#[derive(EntityFields, Debug, Clone)]`
5. Annotate fields with `#[field(...)]`, `#[alias(...)]`, `#[required]`, etc.
6. Add `pub mod my_entity;` to `entity/mod.rs`
7. Add explicit re-export: `pub use my_entity::MyEntity;`
   (do NOT use glob `pub use my_entity::*` — causes name collisions)
8. Run `cargo check` to verify

## How to Add a Field to an Existing Entity

1. Add the Rust field to the struct
2. Add `#[field(display = "...", description = "...")]`
3. Optionally add `#[alias("...")]`, `#[required]`, `#[indexable(priority = N)]`
4. For custom types, use `#[computed_field(...)]` with `#[read(...)]` / `#[write(...)]`
   and explicit closure type annotations
5. Run `cargo check`

## Known Limitations

- Computed field closures must have explicit type annotations on parameters
- `debug_tests.rs` in schedule-macro is disabled (macro emits `crate::` paths
  that only resolve in schedule-data); integration tests belong in schedule-data
- `DefaultFieldUpdater` is stubbed out (`unimplemented!()`)
- `FieldSet.indexable_fields` parsing is in the macro but IndexableField impls
  are not yet auto-generated — currently hand-implemented on entity modules
- `#[validate]` attribute is parsed but validator generation is not yet wired up
