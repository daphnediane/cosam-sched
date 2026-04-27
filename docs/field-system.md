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

### ID Trait Hierarchy

All ID types implement a common trait hierarchy defined in `entity_id.rs` and
`field_node_id.rs`, enabling APIs to accept any ID type uniformly:

```text
EntityUuid            entity_uuid() -> NonNilUuid
EntityTyped           entity_type_name() -> &'static str
DynamicEntityId       blanket impl for EntityUuid + EntityTyped  (for any entity or field node ID)
TypedEntityId<E>      marker for compile-time typed IDs (EntityId<E>, FieldNodeId<E>)

DynamicFieldNodeId    extends DynamicEntityId; adds field() and try_as_typed_field<E>()
TypedFieldNodeId<E>   extends DynamicFieldNodeId + TypedEntityId<E>; adds typed_field()
```

This lets functions accept `impl DynamicEntityId`, `impl TypedEntityId<E>`, or
`impl DynamicFieldNodeId` instead of concrete types, eliminating redundant overloads.

### EntityId\<E\>

`EntityId<E>` is a `Copy + Clone + Hash + Eq` newtype wrapping a `NonNilUuid` with
`PhantomData<fn() -> E>`. `Clone`/`Copy` are manual to avoid spurious
`E: Clone`/`E: Copy` bounds. Implements `EntityUuid`, `EntityTyped`, and
`TypedEntityId<E>`.

Constructors:

```rust
pub fn from_preference(pref: UuidPreference) -> Self;    // primary; resolves via E::uuid_namespace()
pub unsafe fn new_unchecked(uuid: NonNilUuid) -> Self;   // caller must verify type belongs to E
pub fn try_from_dynamic(id: impl DynamicEntityId) -> Option<Self>;  // type-checked conversion
pub fn from_typed<T: TypedEntityId<E>>(id: T) -> Self;  // infallible from typed ID
```

Access: `.entity_uuid() -> NonNilUuid` (via `EntityUuid` trait).

Implements `Serialize`/`Deserialize` (format: `"type_name:uuid"`; rejects nil and wrong type).
Implements `From<EntityId<E>> for RuntimeEntityId` and `TryFrom<RuntimeEntityId> for EntityId<E>`.

### NonNilUuid

`uuid::NonNilUuid` from the `uuid` crate — no custom wrapper needed.
Constructors: `NonNilUuid::new(uuid) -> Option<Self>` and
`unsafe NonNilUuid::new_unchecked(uuid)`.

### RuntimeEntityId

`RuntimeEntityId` — untyped pair `(NonNilUuid, &'static str type_name)` for
dynamic contexts (change-log entries, mixed-kind search). Fields are private;
access via the `EntityUuid` and `EntityTyped` traits. Implements
`Copy + Clone + Hash + Eq + Serialize + Deserialize + Display` (`"type_name:uuid"`).

Constructors:

- `unsafe RuntimeEntityId::new_unchecked(uuid, type_name)` — caller must ensure correspondence
- `RuntimeEntityId::from_dynamic(impl DynamicEntityId)` — safe; converts any ID type
- `From<EntityId<E>>` — safe infallible conversion
- `TryFrom<RuntimeEntityId> for EntityId<E>` — returns `Err(ConversionError)` if type names differ

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

Universal value enum used for all field read/write operations. The system uses a two-level structure. See `conversion-and-lookup.md` for the type-safe conversion system including entity resolution support with `FieldValueForSchedule`.

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

## FieldTypeItem and FieldType

Type-level mirrors of `FieldValueItem` and `FieldValue` for compile-time type declarations.

**`FieldTypeItem`** - Scalar type tags (Copy):

| Variant            | Meaning                           |
| ------------------ | --------------------------------- |
| `String`           | Short text, codes, URLs           |
| `Text`             | Long prose (CRDT RGA routing)     |
| `Integer`          | Counts, durations in minutes      |
| `Float`            | Fractional values                 |
| `Boolean`          | Flags                             |
| `DateTime`         | ISO-8601 timestamps               |
| `Duration`         | Chrono durations                  |
| `EntityIdentifier` | Entity reference (with type name) |

**`FieldType`** - Cardinality wrappers (Copy):

| Variant                   | Use                                            |
| ------------------------- | ---------------------------------------------- |
| `Single(FieldTypeItem)`   | Required single-value fields                   |
| `Optional(FieldTypeItem)` | Optional single-value fields (type-level only) |
| `List(FieldTypeItem)`     | Multi-value fields and relationship lists      |

`FieldType` retains an `Optional` variant because type declarations need to distinguish
"required scalar" from "optional scalar". At the value level, absence is expressed as
`Option<FieldValue>` returning `None` (or `FieldValue::List(vec![])` for the clear sentinel).

These enums enable compile-time type reflection without requiring runtime values.
They are used by converters, importers, and UI code to determine what type a field expects
without calling read/write.

### FieldType methods

- `FieldType::of(&FieldValue) -> Option<Self>` - infer type from a value (returns `None` for empty lists)
- `FieldType::item_type() -> FieldTypeItem` - extract the scalar item type, discarding cardinality
- `FieldType::is_single()`, `is_optional()`, `is_list()` - cardinality predicates

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
VerifiableField<E>  verify(EntityId<E>, &Schedule, &FieldValue) → Result<(), VerificationError>
```

All four traits are flat — no `Simple*` or `Schedule*` sub-traits. The
caller-facing API is always `(EntityId<E>, &[mut] Schedule)`.

`FieldDescriptor<E>` implements all four directly. Dispatch between
data-only and schedule-aware paths is handled internally by matching on
`ReadFn<E>`, `WriteFn<E>`, and `VerifyFn<E>` (see below).

Entity-level text matching (previously `IndexableField<E>` / `match_index`)
is now handled by the `EntityMatcher` trait in `crate::lookup`; see
`conversion-and-lookup.md`. Individual field descriptors no longer carry
an `index_fn` — each entity type owns its holistic `match_entity` logic.

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
    pub field_type: FieldType,          // Logical field type (value type and cardinality)
    pub example: &'static str,
    pub order: u32,                     // Stable iteration order for inventory collection
    pub read_fn: Option<ReadFn<E>>,     // None → write-only
    pub write_fn: Option<WriteFn<E>>,   // None → read-only
    pub verify_fn: Option<VerifyFn<E>>, // None → no verification requested
}
```

The `order: u32` field enables stable field ordering when fields self-register via
inventory. Fields are sorted by this value (ascending) when `FieldSet::from_inventory()`
collects them. Use multiples of 100 (0, 100, 200, ...) to leave room for future insertions.

`FieldDescriptor` implements `NamedField`, `ReadableField<E>`,
`WritableField<E>`, and `VerifiableField<E>` directly:

- `read()` matches `read_fn`: `None` → `FieldError::WriteOnly`;
  `Bare` fetches `InternalData` from the schedule then calls the fn;
  `Schedule` delegates directly.
- `write()` matches `write_fn`: `None` → `FieldError::ReadOnly`;
  `Bare` fetches `&mut InternalData` then calls the fn;
  `Schedule` delegates directly (no double `&mut`).
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
    field_type: FieldType::Single(FieldTypeItem::String),
    example: "Introduction to Cosplay",
    order: 0,
    read_fn: Some(ReadFn::Bare(|d| Some(FieldValue::String(d.data.name.clone())))),
    write_fn: Some(WriteFn::Bare(|d, v| { d.data.name = v.into_string()?; Ok(()) })),
    verify_fn: None,
};

// Edge field (CRDT owner, no direct write fn — write via add/remove helpers):
define_field! {
    static FIELD_PRESENTERS: FieldDescriptor<PanelEntityType>,
    edge: ro, target: PresenterEntityType, target_field: &crate::presenter::FIELD_PANELS, owner,
    name: "presenters", display: "Presenters",
    desc: "All presenters attached to this panel (credited and uncredited).",
    aliases: &["panelists", "presenter"],
    example: "[]",
    order: 2700
}
```

### Field declaration via `define_field!` proc-macro

All field descriptors are declared using the unified `define_field!` function-like
proc-macro from the `schedule-macro` crate (re-exported as `schedule_core::define_field`).
The macro supports three declaration modes:

**Stored fields** (scalar fields backed by `CommonData` slots):

```rust
define_field! {
    static FIELD_NAME: FieldDescriptor<PresenterEntityType>,
    accessor: name, required, as: AsString,
    name: "name", display: "Name",
    desc: "Presenter or group display name.",
    aliases: &["presenter_name", "display_name"],
    example: "Alice Example",
    order: 0
}
```

**Edge fields** (relationship fields):

```rust
define_field! {
    static FIELD_MEMBERS: FieldDescriptor<PresenterEntityType>,
    edge: rw, target: PresenterEntityType, target_field: &FIELD_GROUPS, owner,
    name: "members", display: "Members",
    desc: "Members of this group (empty for individuals).",
    aliases: &["group_members"],
    example: "[]",
    order: 800
}
```

**Custom fields** (computed fields with explicit read/write closures):

```rust
define_field! {
    static FIELD_RANK: FieldDescriptor<PresenterEntityType>,
    name: "rank", display: "Rank",
    desc: "Presenter classification tier.",
    aliases: &["classification"],
    example: "guest",
    order: 100,
    crdt: Scalar, cardinality: optional, item: FieldTypeItem::String,
    read: |d: &PresenterInternalData| {
        Some(field_value!(d.data.rank.as_str()))
    },
    write: |d: &mut PresenterInternalData, v: FieldValue| {
        d.data.rank = PresenterRank::parse(&v.into_string()?);
        Ok(())
    }
}
```

The `accessor:` syntax for stored fields derives the field type and read/write
functions automatically from the `as:` marker trait (`AsString`, `AsBoolean`,
`AsInteger`, `AsText`). The `edge:` syntax generates the appropriate edge-backed
read/write closures. Custom fields explicitly provide their own closures.

See `schedule-macro` crate documentation for the full grammar and all supported
options.

### Edge ownership in `CrdtFieldType` (FEATURE-070)

There is no separate `EdgeDescriptor` struct.  CRDT-edge information lives
directly inside the owner field's `CrdtFieldType`:

```rust
pub enum CrdtFieldType {
    Scalar,
    Text,
    List,
    Derived,
    /// CRDT-canonical owner side of an edge relationship.
    EdgeOwner {
        /// Inverse/lookup field on the target entity.
        target_field: &'static dyn NamedField,
    },
    /// Non-owner (inverse/lookup) side; no payload.
    EdgeTarget,
}
```

Resolution: `edge_crdt::canonical_owner(near, far)` checks each side's
`crdt_type()` — whichever side is `EdgeOwner { target_field }` pointing at the
other is the owner.  Constant time, no inventory traversal.

Per-edge metadata schema (`EdgeFieldSpec` / `EdgeFieldDefault`) was removed in
FEATURE-070.  The `_meta` map storage and `Schedule::edge_get_bool` /
`edge_set_bool` access points still exist for the legacy `credited` flag
(default `true`); FEATURE-065 will retire them entirely by splitting
`presenters` into `credited_presenters` / `uncredited_presenters` first-class
CRDT lists. See `crdt-design.md` for the document path layout.

**Presenter partition fields on Panel** use `define_field!` with `WriteFn::Schedule`
and call `field_value_to_entity_ids` (the standard edge-parsing helper) for input
normalization, then `edge_get_bool` / `edge_set_bool` / `edge_add` / `edge_remove`:

| Field                       | Mode       | Semantics                                                                       |
| --------------------------- | ---------- | ------------------------------------------------------------------------------- |
| `presenters`                | read-only  | All attached presenters (both partitions)                                       |
| `credited_presenters`       | read/write | Read: credited subset. Write: replace credited partition (absent → removed)     |
| `uncredited_presenters`     | read/write | Read: uncredited subset. Write: replace uncredited partition (absent → removed) |
| `add_credited_presenters`   | write-only | Add presenters and set `credited = true`                                        |
| `add_uncredited_presenters` | write-only | Add presenters and set `credited = false`                                       |
| `remove_presenters`         | write-only | Remove presenters from the panel entirely                                       |

The two partitions are **independent**: writing `credited_presenters` does not
affect presenters in the uncredited partition and vice versa. Moving a presenter
between partitions (by writing them into the other field) sets the flag; their
edge is retained.

### Read-only computed fields with Schedule access

For computed fields that require Schedule access to traverse edges or access other
entities, use `ReadFn::Schedule` with a closure that receives `(&Schedule, EntityId<E>)`:

```rust
define_field!(
    static FIELD_CREDITS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
        name: "credits",
        display: "Credits",
        description: "Formatted presenter credit strings for display.",
        aliases: &["credit"],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::List, FieldTypeItem::String),
        example: "[\"John Doe\", \"Group Name (Alice, Bob)\"]",
        order: 3600,
        read_fn: Some(ReadFn::Schedule(
            |sched: &Schedule, id: PanelId| {
                // Access schedule to traverse edges, look up entities, etc.
                let node = FieldNodeId::new(id, &FIELD_PRESENTERS);
                let presenter_ids = sched.connected_entities::<PanelEntityType, PresenterEntityType>(node, &FIELD_PANELS);
                // ... compute and return FieldValue
            },
        )),
        write_fn: None,  // Read-only
        verify_fn: None,
    }
);
```

This pattern is used for:

- **Panel**: `credits` (formats credited presenter strings with group resolution; filtered
  by per-edge `credited` flag), `credited_presenters` / `uncredited_presenters` (read
  the credited/uncredited partitions by checking `edge_get_bool`), `hotel_rooms`
  (traverses event_rooms to hotel rooms)
- **Presenter**: `inclusive_groups`, `inclusive_members` (transitive closure via
  `inclusive_edges_from`/`inclusive_edges_to`)

### Inventory-based field registration

Field descriptors self-register globally via the `inventory` crate. This eliminates
manual `FieldSet::new(&[...])` lists and prevents accidentally
omitting fields from the registry.

```rust
pub struct CollectedNamedField(pub &'static dyn NamedField);
```

The `define_field!` proc-macro automatically generates the `inventory::submit!` call for each field descriptor:

```rust
inventory::submit! { CollectedNamedField(&FIELD_NAME) }
```

The global registry enables type-safe downcasting via `std::any::Any::downcast_ref`,
eliminating the need for per-entity-type registries.

**NamedField trait** - Base trait providing field metadata:

```rust
pub trait NamedField: 'static + Send + Sync + std::any::Any {
    fn name(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str];
    fn field_id(&self) -> FieldId;
    fn entity_type_name(&self) -> &'static str;
}
```

The `field_id()` and `entity_type_name()` methods enable type-erased field identity
and entity type identification. The `std::any::Any` super-trait enables safe downcasting
via `downcast_ref`.

**Inventory registration** - The `define_field!` proc-macro automatically generates
the required `inventory::submit!` call, ensuring every field descriptor is registered
in the global `CollectedNamedField` registry.

## FieldSet

`FieldSet<E>` is an ordered, name-indexed collection of `&'static FieldDescriptor<E>`
values for one entity type. Assembled in a `LazyLock` and returned by
`EntityType::field_set()`. Supports:

- Lookup by canonical name or alias (`get_by_name`) — **exact match, no normalization**
- Iteration in declaration order (`fields()`)
- Partitioned iterators: `required_fields()`, `readable_fields()`, `writable_fields()`
- CRDT field list: `crdt_fields()` — `(name, CrdtFieldType)` for non-`Derived` fields
- Dispatch helpers: `read_field_value(name, id, schedule)`, `write_field_value(name, id, schedule, value)`

Entity-level text matching is no longer part of the `FieldSet` API — it is
provided by the `EntityMatcher` trait on the entity type; see
`conversion-and-lookup.md`.

### Construction via inventory

Production entity types use `FieldSet::from_inventory()` to collect all fields
submitted via the global `inventory::submit! { CollectedNamedField(&FIELD_NAME) }`
registry. Fields are filtered by entity type name, downcast to the concrete
`FieldDescriptor<E>` type via `std::any::Any::downcast_ref`, and sorted by the
`order: u32` field for stable iteration order.

```rust
static FIELD_SET: LazyLock<FieldSet<PanelEntityType>> =
    LazyLock::new(|| FieldSet::from_inventory());
```

`FieldSet::new(&[...])` is kept for test-only mock entities that don't use
inventory collections.

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

## FieldRef

`FieldRef<E>` is an enum for flexibly referencing fields in batch operations
(FEATURE-046: Bulk Field Updates). It allows API consumers to use either
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

## IntoFieldValue Trait Hierarchy

Type-deduced `FieldValue` construction via Rust's trait system.

**IntoFieldValueItem** - Convert Rust types to `FieldValueItem`:

```rust
pub trait IntoFieldValueItem {
    fn into_field_value_item(self) -> FieldValueItem;
}
```

Implemented for: `String`, `&str`, `i64`, `i32`, `f64`, `bool`, `NaiveDateTime`, `Duration`, `RuntimeEntityId`.

**Note**: `Text` is intentionally excluded. The `String` vs `Text` distinction is a storage-layer semantic (LWW vs RGA CRDT), not derivable from a Rust type. Use `FieldValueItem::Text` or `field_text!` explicitly for prose fields.

**IntoFieldValue** - Convert Rust types to `FieldValue` (with cardinality):

```rust
pub trait IntoFieldValue {
    fn into_field_value(self) -> FieldValue;
}
```

Blanket impls:

- `T: IntoFieldValueItem` → `Single(T)`
- `Option<T: IntoFieldValueItem>` → `Single(T)` if `Some`, `List([])` if `None` (clear sentinel)
- `Vec<T: IntoFieldValueItem>` → `List([...])`

### Field value construction macros

Three macros cover all normal cases for creating `FieldValue` instances:

**`field_value!`** - Type-deduced construction via `IntoFieldValue`:

```rust
// Type-deduced single values
field_value!("hello")               // → FieldValue::Single(String("hello"))
field_value!(42i64)                 // → FieldValue::Single(Integer(42))
field_value!(true)                  // → FieldValue::Single(Boolean(true))
field_value!(dt)                    // → FieldValue::Single(DateTime(dt))
field_value!(dur)                   // → FieldValue::Single(Duration(dur))

// Option handling
field_value!(Some("x"))             // → FieldValue::Single(String("x"))
field_value!(Option::<&str>::None)  // → FieldValue::List([]) (clear sentinel)

// Vec handling
field_value!(vec![1i64, 2, 3])      // → FieldValue::List([Integer(1), Integer(2), Integer(3)])

// Empty list
field_value!(empty_list)            // → FieldValue::List([])
```

**`field_text!`** - Explicit `Text` variant for long prose:

```rust
field_text!("long description")     // → FieldValue::Single(Text("long description"))
```

Use `field_text!` when the field uses `CrdtFieldType::Text` (long prose routed to RGA CRDT storage). The Rust type `String` alone is insufficient to distinguish `String` from `Text` since they share the same type but have different CRDT semantics.

**`field_empty_list!`** - Shorthand for empty list:

```rust
field_empty_list!()                 // → FieldValue::List([])
```

## Error Types

- `FieldError` — top-level error for field operations (wraps sub-errors)
- `ConversionError` — type conversion failures (wrong variant, parse failure)
- `ValidationError` — value fails field constraints
- `FieldSetError` — batch write errors (duplicates, unknown fields, write failures, verification failures)

All use `thiserror`.

## Builder System

The builder pattern provides ergonomic entity construction with typed setters,
UUID assignment, validation, and rollback semantics (FEATURE-017). It layers on
top of `FieldSet::write_multiple` for atomic batch field updates.

### EntityBuildable trait

Subtrait of `EntityType` that entity types implement to support building:

```rust
pub trait EntityBuildable: EntityType {
    /// Produce an empty `InternalData` stamped with the given ID.
    fn default_data(id: EntityId<Self>) -> Self::InternalData;
}
```

All fields in the returned `InternalData` are initialized to sensible defaults
(typically via `Default::default()` on the inner `FooCommonData`). Required
fields will intentionally fail `EntityType::validate` until the builder's batch
writes run — this is the mechanism that enforces the "you must set required
fields" contract.

Implemented by all production entity types: `PanelTypeEntityType`,
`PanelEntityType`, `PresenterEntityType`, `EventRoomEntityType`,
`HotelRoomEntityType`.

### build_entity driver

Core function that seeds, populates, and validates a new entity:

```rust
pub fn build_entity<E: EntityBuildable>(
    schedule: &mut Schedule,
    uuid_pref: UuidPreference,
    updates: Vec<(FieldRef<E>, FieldValue)>,
) -> Result<EntityId<E>, BuildError>;
```

**Steps:**

1. Resolve `uuid_pref` to a typed `EntityId<E>` (v7, v5, or exact)
2. Insert `EntityBuildable::default_data` into `schedule`
3. Call `FieldSet::write_multiple` with `updates`
4. Run `EntityType::validate` on the final internal data

On any failure (batch write error or validation error), the placeholder entity
is removed via `Schedule::remove_entity` (which also clears edges), ensuring
rollback semantics.

### define_entity_builder! macro

`macro_rules!` macro in `field_macros.rs` that generates a typed builder with
`with_*` setters per field. This is the only remaining `macro_rules!` macro in
`field_macros.rs` after the migration to the `define_field!` proc-macro. The generated builder:

- Collects field updates in a `Vec<(FieldRef<E>, FieldValue)>`
- Delegates to `build_entity` for seed, write, validate, and rollback
- Accepts a `UuidPreference` parameter for UUID assignment control
- Provides typed setters accepting native Rust types via `IntoFieldValue`

**Usage pattern:**

```rust
define_entity_builder!(
    PanelTypeBuilder,
    PanelEntityType,
    PanelTypeInternalData,
    [
        (FIELD_PREFIX, with_prefix),
        (FIELD_HAS_TRACKING, with_has_tracking),
        // ... more fields
    ]
);
```

Each field tuple specifies the field descriptor static and the setter method
name. The macro generates:

- A `new(uuid_pref: UuidPreference)` constructor
- A `with_<field_name>(value)` setter for each field (accepts `impl IntoFieldValue`)
- A `build(schedule: &mut Schedule) -> Result<EntityId<E>, BuildError>` method

### Instantiated builders

Five production entity builders are instantiated:

- `PanelTypeBuilder` — comprehensive unit tests in `panel_type.rs`
- `PanelBuilder` — scalar fields (duration) and edge fields (presenters, event rooms, panel type)
- `PresenterBuilder` — name, rank, bio, status flags, and edge fields (groups, members, panels)
- `EventRoomBuilder` — name, long_name, sort_key, hotel_rooms, panels
- `HotelRoomBuilder` — name, event_rooms

### BuildError

Error enum returned by `build_entity` (and therefore by every generated builder):

```rust
pub enum BuildError {
    FieldSet(#[from] FieldSetError),      // batch write or verification failed
    Validation(Vec<ValidationError>),    // entity validation failed
}
```

Both variants trigger rollback via `Schedule::remove_entity` before the error
is returned.
