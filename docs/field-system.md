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
pub trait EntityType: 'static + Sized {
    type InternalData: Clone + Send + Sync + fmt::Debug + 'static;
    type Data: Clone;

    const TYPE_NAME: &'static str;
    fn uuid_namespace() -> &'static Uuid;  // v5 namespace derived from TYPE_NAME
    fn field_set() -> &'static FieldSet<Self>;
    fn export(internal: &Self::InternalData) -> Self::Data;
    fn validate(data: &Self::InternalData) -> Vec<ValidationError>;
}
```

No `type Id` — use `EntityId<E>` directly everywhere a compile-time typed ID
is needed. `RuntimeEntityId` covers the untyped/dynamic case.

Entity types: `PanelTypeEntityType`, `PanelEntityType`, `PresenterEntityType`,
`EventRoomEntityType`, `HotelRoomEntityType`.

## Entity Identity

### EntityId\<E\>

`EntityId<E>` is a `Copy + Clone + Hash + Eq` newtype wrapping a `Uuid` with
`PhantomData<fn() -> E>`. `Clone`/`Copy` are manual to avoid spurious
`E: Clone`/`E: Copy` bounds.

Constructors:

```rust
pub fn from_preference(pref: UuidPreference) -> Self;  // primary; resolves via E::UUID_NAMESPACE
pub fn new(uuid: Uuid) -> Option<Self>;                 // None if nil; for deserialization
pub unsafe fn from_uuid(uuid: NonNilUuid) -> Self;      // caller must verify type
pub fn uuid(&self) -> Uuid;
pub fn non_nil_uuid(&self) -> NonNilUuid;               // safe: all constructors uphold non-nil
```

Implements `Serialize`/`Deserialize` (rejects nil on deserialization).

### NonNilUuid

`uuid::NonNilUuid` from the `uuid` crate — no custom wrapper needed.
Constructors: `NonNilUuid::new(uuid) -> Option<Self>` and
`unsafe NonNilUuid::new_unchecked(uuid)`.

### RuntimeEntityId

`RuntimeEntityId { uuid: NonNilUuid, type_name: String }` — untyped pair for
dynamic contexts (change-log entries, mixed-kind search). Implements
`Clone + Hash + Eq + Serialize + Deserialize + Display` (`"TypeName:uuid"`).

- `from_typed<E>(EntityId<E>)` — safe constructor from a typed ID
- `unsafe from_uuid(NonNilUuid, type_name)` — caller must ensure correspondence
- `try_as_typed<E>()` — returns `Some(EntityId<E>)` if type names match

### UuidPreference

Builder-level control over UUID assignment:

| Variant                   | Behavior                                                |
| ------------------------- | ------------------------------------------------------- |
| `GenerateNew` *(default)* | Fresh v7 UUID                                           |
| `FromV5 { name }`         | Deterministic v5 UUID from `E::uuid_namespace()` + name |
| `Exact(NonNilUuid)`       | Round-trip exact UUID                                   |

Resolution is performed by `EntityId::from_preference(UuidPreference) -> Self`
which uses the entity type's `uuid_namespace()` for v5 generation.

## FieldValue

Universal value enum used for all field read/write operations. The system uses a two-level structure:

**`FieldValueItem`** - Scalar value types:

| Variant            | Use                                                |
| ------------------ | -------------------------------------------------- |
| `String`           | Short text, codes, URLs                            |
| `Text`             | Long prose — distinct variant for CRDT RGA routing |
| `Integer`          | Counts, durations in minutes, sort keys            |
| `Float`            | Fractional values                                  |
| `Boolean`          | Flags                                              |
| `DateTime`         | ISO-8601 timestamps                                |
| `Duration`         | Chrono durations                                   |
| `EntityIdentifier` | Entity reference (RuntimeEntityId)                 |

**`FieldValue`** - Cardinality wrapper:

| Variant                     | Use                                       |
| --------------------------- | ----------------------------------------- |
| `Single(FieldValueItem)`    | Single value fields                       |
| `List(Vec<FieldValueItem>)` | Multi-value fields and relationship lists |

Absent optional fields return `None` from read functions; empty lists return `FieldValue::List(vec![])`.

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
VerifiableField<E>  verify(EntityId<E>, &Schedule, &FieldValue) → Result<(), VerificationError>
```

All five traits are flat — no `Simple*` or `Schedule*` sub-traits. The
caller-facing API is always `(EntityId<E>, &[mut] Schedule)`.

`FieldDescriptor<E>` implements all five directly. Dispatch between
data-only and schedule-aware paths is handled internally by matching on
`ReadFn<E>`, `WriteFn<E>`, and `VerifyFn<E>` (see below).

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

/// How a field verifies its value after a batch write: directly from
/// [`EntityType::InternalData`], via a [`Schedule`] lookup, or by re-reading.
pub enum VerifyFn<E: EntityType> {
    /// Data-only verification — no schedule access needed.
    Bare(fn(&E::InternalData, &FieldValue) -> Result<(), VerificationError>),
    /// Schedule-aware verification — fn receives `(&Schedule, EntityId<E>)`.
    Schedule(fn(&Schedule, EntityId<E>, &FieldValue) -> Result<(), VerificationError>),
    /// Re-read verification — read the field back and compare to attempted value.
    /// Uses `read_fn` internally; fails verification if field is write-only.
    ReRead,
}
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
    pub read_fn: Option<ReadFn<E>>,     // None → write-only
    pub write_fn: Option<WriteFn<E>>,   // None → read-only
    pub index_fn: Option<IndexFn<E>>,   // None → not indexable
    pub verify_fn: Option<VerifyFn<E>>, // None → no verification requested
}
```

`FieldDescriptor` implements `NamedField`, `ReadableField<E>`,
`WritableField<E>`, `IndexableField<E>`, and `VerifiableField<E>` directly:

- `read()` matches `read_fn`: `None` → `FieldError::WriteOnly`;
  `Bare` fetches `InternalData` from the schedule then calls the fn;
  `Schedule` delegates directly.
- `write()` matches `write_fn`: `None` → `FieldError::ReadOnly`;
  `Bare` fetches `&mut InternalData` then calls the fn;
  `Schedule` delegates directly (no double `&mut`).
- `match_field()` calls `index_fn` if present.
- `verify()` checks value stability after batch writes (opt-in):
  - If `verify_fn` is `None`, returns `Ok(())` — no verification requested
  - If `verify_fn` is `Some(Bare(f))`, calls the custom data-only verification function
  - If `verify_fn` is `Some(Schedule(f))`, calls the custom schedule-aware function
  - If `verify_fn` is `Some(ReRead)`, reads the field back via `read()` and compares
    to the attempted value; returns `VerificationError::NotVerifiable` if the field
    is write-only (no `read_fn`)
  - Returns `VerificationError::ValueChanged` if the verified value differs from
    the attempted value

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
    verify_fn: None,
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
    verify_fn: None,
};
```

### Shared declaration macros

Uniformly-shaped descriptors (required/optional strings, booleans, optional
integers, plain-text fields, edge-backed stubs) are declared via shared
`macro_rules!` helpers in `crates/schedule-core/src/field_macros.rs`:
`req_string_field!`, `opt_string_field!`, `opt_text_field!`, `bool_field!`,
`opt_i64_field!`, `edge_list_field!`, `edge_list_field_rw!`,
`edge_none_field_rw!`, `edge_mutator_field!`. Each macro takes the entity type
and `InternalData` type explicitly, and assumes the `data: CommonData`
convention. Bespoke descriptors (computed projections, fields with custom
parse logic) stay as hand-written struct literals.

## FieldSet

`FieldSet<E>` is an ordered, name-indexed collection of `&'static FieldDescriptor<E>`
values for one entity type. Assembled manually in a `LazyLock` and returned by
`EntityType::field_set()`. Supports:

- Lookup by canonical name or alias (`get_by_name`) — **exact match, no normalization**
- Iteration in declaration order (`fields()`)
- Partitioned iterators: `required_fields()`, `indexable_fields()`, `readable_fields()`, `writable_fields()`
- CRDT field list: `crdt_fields()` — `(name, CrdtFieldType)` for non-`Derived` fields
- Dispatch helpers: `read_field_value(name, id, schedule)`, `write_field_value(name, id, schedule, value)`
- Index matching: `match_index(query, data)` — best `MatchPriority` across all indexable fields

### Alias registration for XLSX import

`get_by_name` performs exact matching only. The XLSX import layer normalizes
raw column headers before lookup using these steps:

1. Split camelCase lower→upper boundaries (`PanelKind` → `Panel Kind`)
2. Split uppercase-run/UpperCamelCase boundaries (`AVNotes` → `AV Notes`)
3. Collapse whitespace, underscores, and punctuation to `_` and trim

Examples: `"PanelKind"` → `"Panel_Kind"`, `"AVNotes"` → `"AV_Notes"`,
`"Room Name"` → `"Room_Name"`.

**Field authors must register the normalized form as an alias** on any
`FieldDescriptor` that is importable from a spreadsheet. For example, a field
with canonical name `"kind"` whose spreadsheet header is `"PanelKind"` should
include `"Panel_Kind"` in `aliases`.

## FieldRef (Pending, See FEATURE-046)

`FieldRef<E>` is an enum for flexibly referencing fields in batch operations
(see FEATURE-046: Bulk Field Updates). It allows API consumers to use either
field name strings (runtime lookup) or direct descriptor references (zero-cost):

```rust
pub enum FieldRef<E: EntityType> {
    /// Field name string (requires lookup in FieldSet).
    Name(&'static str),
    /// Direct field descriptor reference (zero-cost, compile-time checked).
    Descriptor(&'static FieldDescriptor<E>),
}
```

Used by `FieldSet::write_multiple()` to accept mixed field references:

```rust
// Using field names (ergonomic, runtime lookup)
field_set.write_multiple(id, schedule, &[
    ("start_time", start.into()),
    ("end_time", end.into()),
])?;

// Using field descriptors (zero-cost, compile-time checked)
field_set.write_multiple(id, schedule, &[
    (&FIELD_START_TIME, start.into()),
    (&FIELD_END_TIME, end.into()),
])?;
```

The `From` impls allow ergonomic `.into()` at call sites.

**Status**: Design complete, implementation pending FEATURE-046.

## Error Types

- `FieldError` — top-level error for field operations (wraps sub-errors)
- `ConversionError` — type conversion failures (wrong variant, parse failure)
- `ValidationError` — value fails field constraints

All use `thiserror`.
