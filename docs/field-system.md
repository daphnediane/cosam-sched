# Field System

Entity field system design for `schedule-core`: field traits, `FieldDescriptor`,
`FieldValue`, `FieldSet`, and the three-struct entity pattern.

## Design Principles

- **Visible data structs**: `<E>CommonData` and `<E>InternalData` declarations
  are hand-written and always visible in source — no macro hides them.
- **Macro policy**: proc-macros and `macro_rules!` may generate boilerplate
  (trait impls, field accessor singletons, builders) but must not obscure the
  struct definitions themselves.
- **CRDT-readiness**: every field carries a `CrdtFieldType` annotation from day
  one so CRDT storage can be added in Phase 4 without touching entity structs.

## Three-Struct Entity Pattern

Each entity type is expressed as three hand-written structs:

```text
<E>CommonData  (pub)         — serializable user-facing fields (serde derives)
<E>InternalData (pub(crate)) — CommonData + typed UUID (code) + runtime backing
                               (e.g. time_slot for Panel)
<E>Data        (pub)         — export/API view produced by export(&Schedule):
                               CommonData + string code + projected/computed fields
                               + relationship IDs assembled from edge maps
```

`EntityType::InternalData` is the associated type the field system operates on.
`EntityType::Data` is the public export form. External code never constructs
`InternalData` directly.

## EntityType Trait

```rust
pub trait EntityType {
    type InternalData: Clone + Send + Sync + fmt::Debug + 'static;
    type Data: Clone;

    const TYPE_NAME: &'static str;
    fn field_set() -> &'static FieldSet<Self>;
    fn export(internal: &Self::InternalData) -> Self::Data;
    fn validate(data: &Self::InternalData) -> Vec<ValidationError>;
}
```

No `type Id` — use `EntityId<E>` directly everywhere a compile-time typed ID
is needed. `RuntimeEntityId(EntityKind, Uuid)` covers the untyped/dynamic case.

Entity types: `PanelTypeEntityType`, `PanelEntityType`, `PresenterEntityType`,
`EventRoomEntityType`, `HotelRoomEntityType`.

## FieldValue

Universal value enum used for all field read/write operations:

| Variant                 | Use                                                |
| ----------------------- | -------------------------------------------------- |
| `String`                | Short text, codes, URLs                            |
| `Text`                  | Long prose — distinct variant for CRDT RGA routing |
| `Integer`               | Counts, durations in minutes, sort keys            |
| `Float`                 | Fractional values                                  |
| `Boolean`               | Flags                                              |
| `DateTime`              | ISO-8601 timestamps                                |
| `Duration`              | Chrono durations                                   |
| `NonNilUuid`            | Single entity reference                            |
| `List(Vec<FieldValue>)` | Multi-value fields and relationship lists          |
| `EntityIdentifier`      | UUID or string tag for entity lookup               |
| `None`                  | Absent / unset                                     |

## CrdtFieldType

Annotation on every `FieldDescriptor` controlling how the field maps to CRDT
storage (Phase 4):

| Variant   | Semantics                                                      |
| --------- | -------------------------------------------------------------- |
| `Scalar`  | Last-write-wins via `put_scalar` / `read_scalar`               |
| `Text`    | Prose RGA via `splice_text` / `read_text`                      |
| `List`    | OR-Set equivalent via `list_add` / `list_remove` / `read_list` |
| `Derived` | Computed from relationships; NOT stored in CRDT                |

## Field Trait Hierarchy

```text
NamedField          name(), display_name(), description(), aliases()
ReadableField<E>    read(EntityId<E>, &Schedule) → Option<FieldValue>
WritableField<E>    write(EntityId<E>, &mut Schedule, FieldValue) → Result<(), FieldError>
IndexableField<E>   match_field(&str, &InternalData) → Option<MatchPriority>
```

All four traits are flat — no `Simple*` or `Schedule*` sub-traits. The
caller-facing API is always `(EntityId<E>, &[mut] Schedule)`.

`FieldDescriptor<E>` implements all four directly. Dispatch between
data-only and schedule-aware paths is handled internally by matching on
`ReadFn<E>` and `WriteFn<E>` (see below).

## ReadFn / WriteFn enums

Each `FieldDescriptor` carries enum-valued fn pointers that select the
correct calling convention. This avoids any double-`&mut` borrow problem:
the `Schedule` variant never exposes `&mut InternalData` to the caller.

```rust
pub enum ReadFn<E: EntityType> {
    /// Data-only read — no schedule needed.
    Bare(fn(&E::InternalData) -> Option<FieldValue>),
    /// Schedule-aware read — fn looks up entity internally.
    Schedule(fn(&Schedule, EntityId<E>) -> Option<FieldValue>),
}

pub enum WriteFn<E: EntityType> {
    /// Data-only write — no schedule needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware write — used for edge mutations (add_presenters, etc.).
    /// Fn receives (&mut Schedule, EntityId<E>) with no &mut InternalData;
    /// it handles its own lookup/release internally.
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
}

pub type IndexFn<E> = fn(&str, &<E as EntityType>::InternalData) -> Option<MatchPriority>;
```

## FieldDescriptor

Generic struct — one `static` value per field. Non-capturing closures coerce
to fn pointers automatically.

```rust
pub struct FieldDescriptor<E: EntityType> {
    pub name: &'static str,
    pub display: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub required: bool,
    pub crdt_type: CrdtFieldType,
    pub read_fn: Option<ReadFn<E>>,    // None → write-only
    pub write_fn: Option<WriteFn<E>>,   // None → read-only
    pub index_fn: Option<IndexFn<E>>,   // None → not indexable
}
```

`FieldDescriptor` implements `NamedField`, `ReadableField<E>`,
`WritableField<E>`, and `IndexableField<E>` directly:

- `read()` matches `read_fn`: `None` → `FieldError::WriteOnly`;
  `Bare` fetches `InternalData` from the schedule then calls the fn;
  `Schedule` delegates directly.
- `write()` matches `write_fn`: `None` → `FieldError::ReadOnly`;
  `Bare` fetches `&mut InternalData` then calls the fn;
  `Schedule` delegates directly (no double `&mut`).
- `match_field()` calls `index_fn` if present.

Declared as `static` values, e.g.:

```rust
static FIELD_PANEL_NAME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "name",
    display: "Panel Name",
    description: "The title of the panel.",
    aliases: &[],
    required: true,
    crdt_type: CrdtFieldType::Scalar,
    read_fn: Some(ReadFn::Bare(|d| Some(FieldValue::String(d.data.name.clone())))),
    write_fn: Some(WriteFn::Bare(|d, v| { d.data.name = v.into_string()?; Ok(()) })),
    index_fn: None,
};

static FIELD_PANEL_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "presenters",
    display: "Presenters",
    description: "Presenters assigned to this panel.",
    aliases: &["presenter"],
    required: false,
    crdt_type: CrdtFieldType::Derived,
    read_fn: Some(ReadFn::Schedule(|schedule, id| { /* query edge index */ todo!() })),
    write_fn: Some(WriteFn::Schedule(|schedule, id, v| { /* mutate edge index */ todo!() })),
    index_fn: None,
};
```

## FieldSet

`FieldSet<E>` is an ordered, name-indexed collection of `&'static FieldDescriptor<E>`
values for one entity type. Assembled manually in a `LazyLock` and returned by
`EntityType::field_set()`. Supports:

- Lookup by canonical name
- Lookup by alias
- Iteration in declaration order

## Error Types

- `FieldError` — top-level error for field operations (wraps sub-errors)
- `ConversionError` — type conversion failures (wrong variant, parse failure)
- `ValidationError` — value fails field constraints

All use `thiserror`.
