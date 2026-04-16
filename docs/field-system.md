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
    type InternalData: Clone + Send + Sync + fmt::Debug;
    type Data: Clone + Serialize + Deserialize<'_>;
    type Id: TypedId;

    const TYPE_NAME: &'static str;
    fn field_set() -> &'static FieldSet<Self>;
    fn export(internal: &Self::InternalData, schedule: &Schedule) -> Self::Data;
    fn validate(data: &Self::InternalData) -> Vec<ValidationError>;
}
```

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
NamedField                        name(), display_name(), description(), aliases()
├── SimpleReadableField<E>        read(&InternalData) → Option<FieldValue>
│   └── (blanket) ReadableField<E>  read(&InternalData, &Schedule) → Option<FieldValue>
├── SimpleWritableField<E>        write(&mut InternalData, FieldValue) → Result
│   └── (blanket) WritableField<E>  write(&mut InternalData, &mut Schedule, FieldValue)
└── IndexableField<E>             match_field(query, &InternalData) → Option<MatchPriority>
```

Blanket impls promote `Simple*` variants (data-only) to full variants that also
accept a `&Schedule` / `&mut Schedule` context parameter for edge-aware computed
fields.

## FieldDescriptor

Generic struct with fn pointers — one static value per field:

```rust
pub struct FieldDescriptor<E: EntityType> {
    pub name: &'static str,
    pub display: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub required: bool,
    pub crdt_type: CrdtFieldType,
    pub read_fn:  fn(&E::InternalData) -> Option<FieldValue>,
    pub write_fn: Option<fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>>,
}
```

Non-capturing closures coerce to fn pointers. Closures access `internal.data.*`
for `CommonData` fields and `internal.code` / `internal.time_slot` for
internal-only fields.

Declared as `static` values, e.g.:

```rust
static FIELD_PANEL_NAME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "name",
    display: "Panel Name",
    crdt_type: CrdtFieldType::Scalar,
    read_fn: |internal| Some(FieldValue::String(internal.data.name.clone())),
    write_fn: Some(|internal, v| { internal.data.name = v.into_string()?; Ok(()) }),
    ..
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
