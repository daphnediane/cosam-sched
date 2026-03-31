---
trigger: glob
globs: crates/schedule-macro/**/*.rs,crates/schedule-data/**/*.rs
---
# Entity Field System Architecture

Proc-macro-based field system for type-safe entity definitions. Entity structs in `crates/schedule-data/src/entity/` derive `#[derive(EntityFields)]` from `schedule-macro`.

## Crate Layout

- **`schedule-macro`** — `EntityFields` derive macro
- **`schedule-data`** — data model, depends on `schedule-macro`
  - `entity/` — entity structs, `EntityType` trait, `EntityId` (`u64`)
  - `field/` — trait hierarchy, `FieldSet<T>`, `FieldValue` enum

## Trait Hierarchy

```text
NamedField (name, display, description)
├── SimpleReadable<T> (read without schedule)
├── SimpleWritable<T> (write without schedule)  
├── SimpleChecked<T>  (validate without schedule)
├── IndexableField<T> (lookup, priority)
├── ReadableField<T>  (read with &Schedule)
├── WritableField<T>  (write with &Schedule)
└── CheckedField<T>   (validate with &Schedule)

Blanket impls: Simple* → * (ignore schedule parameter)
Combo traits: SimpleField<T> = SimpleReadable + SimpleWritable
               Field<T> = Readable + Writable
```

## EntityType Trait

```rust
pub trait EntityType: 'static + Send + Sync + Sized {
    type Data: Clone + Send + Sync + fmt::Debug;  // usually Self
    const TYPE_NAME: &'static str;                 // e.g. "room"
    fn field_set() -> &'static FieldSet<Self>;
    fn validate(data: &Self::Data) -> Result<(), ValidationError>;
}
```

Macro generates separate `EntityType` struct (e.g., `RoomEntityType`) with `type Data = Room`, `TYPE_NAME` = lowercase struct name, and `LazyLock<FieldSet<EntityType>>` registry.

## Macro Attributes

### Usage

```rust
use schedule_data::entity::{RoomEntityType, Room};

// Access field set via EntityType
let fs = RoomEntityType::field_set();

// Access data struct for storage/manipulation
let room = Room { ... };

// Validate data via EntityType
RoomEntityType::validate(&room)?;
```

### Direct Fields

```rust
#[field(display = "Room Name", description = "Short room name")]
#[alias("short", "room_name")]       // extra lookup names
#[required]                           // validation: must be non-empty/non-None
#[indexable(priority = 180)]          // participate in get_by_index lookups
pub short_name: String,
```

Supported: `String`, `i64`, `i32`, `u64`, `u32`, `bool`, `EntityId`, `Option<T>`, `HashMap<String, FieldValue>`.

### Computed Fields

```rust
#[computed_field(display = "Edge Type", description = "Type of relationship")]
#[alias("type", "edge_type")]
#[read(|entity: &Edge| { ... -> Option<FieldValue> })]
#[write(|entity: &mut Edge, value: FieldValue| { ... -> Result<(), FieldError> })]
pub edge_type: EdgeType,
```

**Critical**: Closure parameters MUST have explicit type annotations. For schedule access, add `schedule` as first parameter: `#[read(|schedule: &Schedule, entity: &Panel| { ... })]`

### Field Naming

- `#[field_name("custom_name")]` — override internal field name
- `#[field_const("CUSTOM_CONST")]` — override generated constant name

## Generated Code

For each field: unit struct, `NamedField` impl, read/write impls, static constant. `pub mod fields { ... }` submodule contains all field structs.

Macro also generates: separate `EntityType` struct (e.g., `RoomEntityType`), `EntityType` impl, `field_set()` with `LazyLock`, `validate()` checking required fields.

## FieldSet and Indexing

`FieldSet<T>` holds: all fields, name/alias map, required fields, indexable fields.

`match_index(query, entity_id, entity)` iterates indexable fields, returns best `FieldMatchResult` ranked by `(strength, priority)`.

`EntityStorage::get_by_index<T>(query)` iterates entities, calls `match_index`, returns `Vec<&T::Data>` at best match strength.

## Edge System

Edges represent relationships. `EdgeType` encodes relationship kind. `from_uid`/`to_uid` are `EntityId` (u64). Previously `Vec<String>` fields, now separate `Edge` entities.

## Adding New Entity

1. Create `entity/my_entity.rs` with copyright header
2. `use crate::EntityFields;`
3. `#[derive(EntityFields, Debug, Clone)]` struct with field annotations
4. Add `pub mod my_entity;` to `entity/mod.rs`
5. Add explicit re-export: `pub use my_entity::MyEntity;` (no glob imports)
6. `cargo check`

## Adding Field to Entity

1. Add Rust field with `#[field(display = "...", description = "...")]`
2. Optionally add `#[alias(...)]`, `#[required]`, `#[indexable(priority = N)]`
3. For custom types: `#[computed_field(...)]` with `#[read(...)]`/`#[write(...)]` and explicit type annotations
4. `cargo check`

## Limitations

- Computed field closures need explicit type annotations
- `debug_tests.rs` disabled (macro emits `crate::` paths)
- `DefaultFieldUpdater` stubbed
- `IndexableField` impls currently hand-implemented
- `#[validate]` parsed but not wired up
