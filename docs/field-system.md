# Field System

Entity field system design for `schedule-core`: field traits, `FieldDescriptor`,
`HalfEdgeDescriptor`, `FieldValue`, `FieldSet`, and the three-struct entity pattern.

## Design Principles

- **Visible data structs**: `<E>CommonData` and `<E>InternalData` declarations
  are hand-written and always visible in source — no macro hides them.
- **Macro policy**: proc-macros and `macro_rules!` may generate boilerplate
  (trait impls, field accessor singletons, builders) but must not obscure the
  struct definitions themselves.
- **CRDT-readiness**: every field carries a `CrdtFieldType` annotation from day
  one so CRDT storage can be added without touching entity structs.

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

All ID types implement a common trait hierarchy defined in `entity/id.rs`,
enabling APIs to accept any ID type uniformly:

```text
EntityUuid            entity_uuid() -> NonNilUuid
EntityTyped           entity_type_name() -> &'static str
DynamicEntityId       blanket impl for EntityUuid + EntityTyped  (for any entity ID)
TypedEntityId<E>      marker for compile-time typed IDs (EntityId<E>)
```

This lets functions accept `impl DynamicEntityId` or `impl TypedEntityId<E>` instead of concrete types, eliminating redundant overloads.

### EntityId\<E\>

`EntityId<E>` is a `Copy + Clone + Hash + Eq` newtype wrapping a `NonNilUuid` with
`PhantomData<fn() -> E>`. `Clone`/`Copy` are manual to avoid spurious
`E: Clone`/`E: Copy` bounds. Implements `EntityUuid`, `EntityTyped`, and
`TypedEntityId<E>`.

Constructors:

```rust
pub fn generate() -> Self;                                     // safe; creates new v7 UUID
pub unsafe fn from_preference_unchecked(pref: UuidPreference) -> Self;  // unsafe; resolves via E::uuid_namespace()
pub unsafe fn new_unchecked(uuid: NonNilUuid) -> Self;         // caller must verify type belongs to E
pub fn try_from_dynamic(id: impl DynamicEntityId) -> Option<Self>;  // type-checked conversion
pub fn from_typed<T: TypedEntityId<E>>(id: T) -> Self;          // infallible from typed ID
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

Resolution is performed by `EntityId::from_preference_unchecked(UuidPreference) -> Self`
which uses the entity type's `uuid_namespace()` for v5 generation. For safe UUID resolution
with conflict checking, use `Schedule::try_resolve_entity_id()`.

## FieldValue

Universal value enum used for all field read/write operations.

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
storage:

| Variant   | Semantics                                                      |
| --------- | -------------------------------------------------------------- |
| `Scalar`  | Last-write-wins via `put_scalar` / `read_scalar`               |
| `Text`    | Prose RGA via `splice_text` / `read_text`                      |
| `List`    | OR-Set equivalent via `list_add` / `list_remove` / `read_list` |
| `Derived` | Computed from relationships; NOT stored in CRDT                |

All edge relationship fields use `CrdtFieldType::Derived`. Edge ownership direction
is encoded in `EdgeKind` (within `HalfEdgeDescriptor`), not in `CrdtFieldType`.

## Field Trait Hierarchy

```text
NamedField          name(), display_name(), description(), aliases(), entity_type_name(), try_as_half_edge()
```

The caller-facing API is always `(EntityId<E>, &[mut] Schedule)`.
Entity-level matching is handled via `crate::query::lookup::EntityMatcher`.

`FieldDescriptor<E>` implements `NamedField` directly.
`HalfEdgeDescriptor` also implements `NamedField` (with `try_as_half_edge()` returning `Some(self)`).

## ReadFn / WriteFn / AddFn / RemoveFn enums

Each `FieldDescriptor` carries enum-valued fn pointers that select the
correct calling convention. This avoids any double-`&mut` borrow problem:
the `Schedule` variant never exposes `&mut InternalData` to the caller.

```rust
pub enum ReadFn<E: EntityType> {
    /// Data-only read — no schedule access needed.
    Bare(fn(&E::InternalData) -> Option<FieldValue>),
    /// Schedule-aware read — fn receives `(&Schedule, EntityId<E>)` and
    /// performs its own entity lookup internally.
    Schedule(fn(&Schedule, EntityId<E>) -> Option<FieldValue>),
}

pub enum WriteFn<E: EntityType> {
    /// Data-only write — no schedule access needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware write — used for edge mutations (e.g. `add_presenters`).
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
}

pub enum AddFn<E: EntityType> {
    /// Data-only add — no schedule access needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware add — used for edge mutations (e.g. `add_presenters`).
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
}

pub enum RemoveFn<E: EntityType> {
    /// Data-only remove — no schedule access needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware remove — used for edge mutations (e.g. `remove_presenters`).
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
}
```

## FieldDescriptor

Generic struct — one `static` value per field. Non-capturing closures coerce
to fn pointers automatically.

```rust
pub struct FieldDescriptor<E: EntityType> {
    pub(crate) data: CommonFieldData,
    pub crdt_type: CrdtFieldType,
    pub required: bool,
    pub(crate) cb: FieldCallbacks<E>,
}
```

`FieldDescriptor<E>` implements `NamedField` directly:

- `read()` matches `read_fn`: `None` → `FieldError::WriteOnly`;
  `Bare` fetches `InternalData` from the schedule then calls the fn;
  `Schedule` delegates directly.
- `write()` matches `write_fn`: `None` → `FieldError::ReadOnly`;
  `Bare` fetches `&mut InternalData` then calls the fn;
  `Schedule` delegates directly (no double `&mut`).
- `add()` matches `add_fn`: `None` → `FieldError::ReadOnly` or add not supported;
  `Bare` fetches `&mut InternalData` then calls the fn;
  `Schedule` delegates directly.
- `remove()` matches `remove_fn`: `None` → `FieldError::ReadOnly` or remove not supported;
  `Bare` fetches `&mut InternalData` then calls the fn;
  `Schedule` delegates directly.

Declared as `static` values, e.g.:

```rust
// Stored field using accessor_field_properties!:
pub static FIELD_PANEL_NAME: FieldDescriptor<PanelEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        PanelEntityType,
        accessor: panel_name,
        as: AsString,
        name: "panel_name",
        display: "Panel Name",
        description: "The title of the panel.",
        aliases: &[],
        cardinality: Single,
        item: String,
        example: "Introduction to Cosplay",
        order: 0,
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: true,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_PANEL_NAME) }
```

## HalfEdgeDescriptor

Edge field descriptor — one `static` value per edge field on an entity type.
Replaces `FieldDescriptor<E>` for edge fields (owner and target sides).

```rust
pub struct HalfEdgeDescriptor {
    pub(crate) data: CommonFieldData,
    pub edge_kind: EdgeKind,
    pub entity_name: &'static str,
}
```

The `edge_kind` field distinguishes ownership and carries the target/source field
references and exclusivity information. Note that `HalfEdgeDescriptor` does not
include `crdt_type` since edge fields are always `Derived` (stored as CRDT lists
on the canonical owner side).

### EdgeKind

Ownership and relationship info for an edge half-edge field:

```rust
pub enum EdgeKind {
    /// Non-owner (lookup/inverse) side of an edge relationship.
    /// `source_fields` lists all owner-side fields whose `target_field` points
    /// at this field.
    Target {
        source_fields: &'static [&'static HalfEdgeDescriptor],
    },
    /// CRDT-canonical owner side of an edge relationship.
    /// `exclusive_with` names a sibling field on the *same* entity whose
    /// entries must be removed before adding to this field.
    Owner {
        target_field: &'static HalfEdgeDescriptor,
        exclusive_with: Option<&'static HalfEdgeDescriptor>,
    },
}
```

Resolution: `crdt::edge::canonical_owner(near, far)` checks each side's
`edge_kind()` — whichever side is `EdgeKind::Owner { target_field, .. }`
pointing at the other is the owner. Constant time, no inventory traversal.

Edge fields are distinguished from regular fields via `NamedField::try_as_half_edge()`,
which returns `Some(&HalfEdgeDescriptor)` for edge fields and `None` for regular fields.

### FullEdge

Represents a complete edge between two field descriptors:

```rust
pub struct FullEdge {
    pub near: &'static HalfEdgeDescriptor,
    pub far: &'static HalfEdgeDescriptor,
}
```

`FullEdge` serializes to a compact 2-field JSON object:

```json
{"ownerField": "panel:panel_type", "nearIsOwner": true}
```

- **`ownerField`** — `field_key()` of the Owner half-edge (`"entity_type:field_name"`).
- **`nearIsOwner`** — `true` if `near = owner, far = target`; `false` for the flipped orientation.

Every `FullEdge` has exactly one Owner half-edge, so this form uniquely identifies
any edge — even when a Target half-edge is shared by multiple Owners
(e.g. `presenter:panels` is the target of both `credited_presenters` and
`uncredited_presenters`; they are disambiguated by their distinct owner keys).

## Field Descriptor Macros

Two proc-macros generate the `(CommonFieldData, CrdtFieldType, FieldCallbacks)` tuple for
hand-written field descriptors. Each is designed for a specific field type:

- **`accessor_field_properties!`** — Stored fields backed by `CommonData` slots
- **`callback_field_properties!`** — Computed fields with custom callbacks

**When to use each approach:**

- Use `accessor_field_properties!` for stored fields that map directly to `CommonData` fields
- Write `HalfEdgeDescriptor` directly for edge relationship fields (no macro)
- Use `callback_field_properties!` for computed fields with custom read/write logic

**Macro return pattern:**

Both macros return a 3-tuple `(data, crdt_type, cb)`:

- `data`: `CommonFieldData` with name, display, description, field_type, example, order
- `crdt_type`: Default `CrdtFieldType` derived from field type (List → List, Single/Optional → use marker trait's CRDT_TYPE)
- `cb`: `FieldCallbacks<E>` with read_fn, write_fn, add_fn, remove_fn

Field authors can use the macro's default `crdt_type` or override it by using `let (data, _, cb)` (ignoring the macro's crdt_type) and explicitly setting `crdt_type: CrdtFieldType::Derived` in the `FieldDescriptor` initialization.

### Using `accessor_field_properties!`

For stored fields that map directly to `CommonData` fields, use `accessor_field_properties!`
to auto-generate read/write callbacks from the field's `As*` marker trait:

```rust
pub static FIELD_NAME: FieldDescriptor<PresenterEntityType> = {
    let (data, crdt_type, cb) = accessor_field_properties! {
        PresenterEntityType,
        accessor: name,           // Field name in CommonData
        as: AsString,             // Marker trait for type conversion
        name: "name",
        display: "Name",
        description: "Presenter or group display name.",
        aliases: &["presenter_name", "display_name"],
        example: "Alice Example",
        order: 0
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: true,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_NAME) }
```

**Parameters:**

- `accessor: <ident>` — The field name in `CommonData` (e.g., `name`, `bio`)
- `as: <trait>` — The marker trait for type conversion (`AsString`, `AsBoolean`, `AsInteger`, `AsText`)
- Standard `CommonFieldData` parameters (`name`, `display`, `description`, `aliases`, `example`, `order`)

The macro derives `cardinality` from the field type (via the marker trait),
generates `ReadFn::Bare` and `WriteFn::Bare` callbacks that access the field directly,
and computes a default `crdt_type` based on the field type (List → List, Single/Optional → use marker trait's CRDT_TYPE).

### Writing HalfEdgeDescriptor

Edge relationship fields are declared as static `HalfEdgeDescriptor` values written
without a macro. The descriptor includes the `CommonFieldData`, `EdgeKind` for ownership
direction, and the entity type name:

```rust
pub static HALF_EDGE_PANELS: crate::edge::HalfEdgeDescriptor = {
    crate::edge::HalfEdgeDescriptor {
        data: crate::field::CommonFieldData {
            name: "panels",
            display: "Panels",
            description: "Panels of this type.",
            aliases: &[],
            field_type: crate::value::FieldType(
                crate::value::FieldCardinality::List,
                crate::value::FieldTypeItem::EntityIdentifier(PanelEntityType::TYPE_NAME),
            ),
            example: "[]",
            order: 1200,
        },
        edge_kind: crate::edge::EdgeKind::Target {
            source_fields: &[&panel::HALF_EDGE_PANEL_TYPE],
        },
        entity_name: PanelTypeEntityType::TYPE_NAME,
    }
};
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_PANELS) }
```

**For owner-side edges**, use `EdgeKind::Owner`:

```rust
edge_kind: crate::edge::EdgeKind::Owner {
    target_field: &presenter::HALF_EDGE_PANELS,
    exclusive_with: Some(&HALF_EDGE_UNCREDITED_PRESENTERS),
},
```

**For target-side edges**, use `EdgeKind::Target`:

```rust
edge_kind: crate::edge::EdgeKind::Target {
    source_fields: &[&panel::HALF_EDGE_CREDITED_PRESENTERS, &panel::HALF_EDGE_UNCREDITED_PRESENTERS],
},
```

All edge fields must be registered via `inventory::submit! { CollectedHalfEdge(&FIELD_NAME) }`
after the static definition.

### Using `callback_field_properties!`

For computed fields that need custom callback logic, use the `callback_field_properties!`
macro to generate the `(CommonFieldData, CrdtFieldType, FieldCallbacks)` tuple, then construct
the `FieldDescriptor` manually:

```rust
pub static FIELD_CODE: FieldDescriptor<PanelEntityType> = {
    let (data, crdt_type, cb) = callback_field_properties! {
        PanelEntityType,
        name: "code",
        display: "Uniq ID",
        description: "Panel Uniq ID (e.g. \"GP032\"), parsed from the Schedule sheet.",
        aliases: &["uid", "uniq_id", "id"],
        cardinality: Single,
        item: String,
        example: "GP032",
        order: 0,
        read: |d: &PanelInternalData| {
            Some(field_value!(d.code.full_id()))
        },
        write: |d: &mut PanelInternalData, v: FieldValue| {
            let s = v.into_string()?;
            match PanelUniqId::parse(&s) {
                Some(parsed) => {
                    d.code = parsed;
                    Ok(())
                }
                None => Err(crate::value::ConversionError::ParseError {
                    message: format!("could not parse panel Uniq ID {s:?}"),
                }
                .into()),
            }
        }
    };
    FieldDescriptor {
        data,
        crdt_type,
        required: true,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_CODE) }
```

**Overriding the default crdt_type:**

For fields that should be `Derived` (computed from relationships, not stored in CRDT), use `let (data, _, cb)` to ignore the macro's default and explicitly set `crdt_type: CrdtFieldType::Derived`:

```rust
pub static FIELD_PRESENTERS: FieldDescriptor<PanelEntityType> = {
    let (data, _, cb) = callback_field_properties! {
        PanelEntityType,
        name: "presenters",
        display: "Presenters",
        description: "Read-only union of credited and uncredited presenter lists.",
        aliases: &[],
        cardinality: List,
        item: EntityIdentifier,
        item_entity: PresenterEntityType,
        example: "[]",
        order: 2700,
        read: |sched: &Schedule, id: PanelId| {
            // Computed from edge relationships
            None
        },
    };
    FieldDescriptor {
        data,
        crdt_type: CrdtFieldType::Derived,
        required: false,
        cb,
    }
};
inventory::submit! { CollectedField(&FIELD_PRESENTERS) }
```

## Inventory-based Field Registration

Field descriptors self-register globally via the `inventory` crate. This eliminates
manual `FieldSet::new(&[...])` lists and prevents accidentally
omitting fields from the registry.

```rust
pub struct CollectedField(pub &'static dyn NamedField);
pub struct CollectedHalfEdge(pub &'static dyn NamedField);

inventory::collect!(CollectedField);
inventory::collect!(CollectedHalfEdge);
```

Field descriptors must explicitly call `inventory::submit!` after definition:

```rust
inventory::submit! { CollectedField(&FIELD_NAME) }
inventory::submit! { CollectedHalfEdge(&HALF_EDGE_NAME) }
```

The global registry enables type-safe downcasting via `std::any::Any::downcast_ref`,
eliminating the need for per-entity-type registries.

## CommonFieldData

Generic field data shared by all field descriptors:

```rust
pub struct CommonFieldData {
    pub name: &'static str,
    pub display: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
    pub field_type: FieldType,
    pub example: &'static str,
    pub order: u32,
}
```

Fields are `pub(crate)` so entity modules and macro-generated code within
`schedule-core` can initialize statics using struct literal syntax.
External code accesses these through the `NamedField` trait methods.

The `order: u32` field enables stable field ordering when fields self-register via
inventory. Fields are sorted by this value (ascending) when `FieldSet::from_inventory()`
collects them. Use multiples of 100 (0, 100, 200, ...) to leave room for future insertions.

**Note:** `crdt_type` is not part of `CommonFieldData`; it is a direct field on `FieldDescriptor<E>`
instead. This separation allows `HalfEdgeDescriptor` (which uses `CommonFieldData`) to avoid
carrying a redundant `crdt_type` since edge fields are always `Derived`.

## FieldSet

`FieldSet<E>` is the collection of all field descriptors for a given entity type.
It provides lookup by name and iteration over fields.

```rust
pub struct FieldSet<E: EntityType> {
    fields: Vec<&'static FieldDescriptor<E>>,
    half_edges: Vec<&'static HalfEdgeDescriptor>,
    by_name: HashMap<&'static str, &'static FieldDescriptor<E>>,
}
```

`FieldSet::from_inventory()` constructs a `FieldSet` by filtering the global
registry by entity type name, downcasting each match to the concrete type, and
sorting by `order`.

## Edge Operations API

The `Schedule` struct provides low-level edge manipulation methods:

```rust
pub fn edge_add(
    &mut self,
    near: impl DynamicEntityId,
    edge: FullEdge,
    far_nodes: impl IntoIterator<Item = impl DynamicEntityId>,
) -> Result<Vec<NonNilUuid>, EdgeError>

pub fn edge_remove(
    &mut self,
    near: impl DynamicEntityId,
    edge: FullEdge,
    far_nodes: impl IntoIterator<Item = impl DynamicEntityId>,
) -> Vec<NonNilUuid>

pub fn edge_set(
    &mut self,
    near: impl DynamicEntityId,
    edge: FullEdge,
    targets: impl IntoIterator<Item = impl DynamicEntityId>,
) -> Result<(Vec<NonNilUuid>, Vec<NonNilUuid>), EdgeError>
```

**Key design decisions:**

- **Batch operations**: All methods accept iterators of far nodes, enabling efficient
  addition/removal of multiple edges in a single call.
- **Return actual UUIDs**: All methods return the UUIDs of edges that were actually
  added or removed (not just counts). This enables:
  - Precise exclusive edge cleanup in helper functions
  - CRDT mirroring with incremental updates instead of full rewrites
  - Better observability and debugging
- **Parameter order**: `near` (source), `edge` (relationship), `far_nodes` (targets).

**Efficiency improvements in RawEdgeMap:**

The underlying `RawEdgeMap::add_edge` and `RawEdgeMap::remove_edge` have been
optimized to avoid redundant hash map lookups:

- **Single lookup per operation**: Instead of per-target lookups, the methods now
  do a single hash map lookup for the near side, compute the diff, and then iterate
  only over the changed targets for the reverse direction.
- **Batch diff computation**: The set of actually-added/removed UUIDs is computed
  once, then used for both the near-side update and the far-side reverse update.

## Homogeneous Edges and Explicit `FullEdge` Constants

Homogeneous edges connect entities of the same type (e.g., Presenter-to-Presenter
member/group relationships). Because the field names (`FIELD_MEMBERS`,
`FIELD_GROUPS`) are symmetric and their semantic roles depend on which side is the
"near" node, these edges are prone to near/far confusion.

**Solution:** Define explicit `FullEdge` static constants that name the query intent:

```rust
/// Static edge from groups field to members field (for querying a presenter's groups)
static EDGE_GROUPS: FullEdge = FullEdge {
    near: &FIELD_GROUPS,   // Start at the presenter's "groups" field
    far: &FIELD_MEMBERS,   // Follow to members on the target side
};

/// Static edge from members field to groups field (for querying a group's members)
static EDGE_MEMBERS: FullEdge = FullEdge {
    near: &FIELD_MEMBERS,  // Start at the group's "members" field
    far: &FIELD_GROUPS,    // Follow to groups on the target side
};
```

**Usage in computed fields:**

```rust
// Get all groups this presenter belongs to (transitive)
let ids = sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(id, EDGE_GROUPS);

// Get all members of this group (transitive)
let ids = sched.inclusive_edges::<PresenterEntityType, PresenterEntityType>(id, EDGE_MEMBERS);
```

The `FullEdge` type is defined in `crate::edge::id` and provides `near`/`far`
accessors, direction checking (`is_homogeneous()`), and edge flipping (`flip()`).

## Panel ↔ Presenter Edge Partitions

The Panel ↔ Presenter relationship is split into two independent edge lists
on Panel: `credited_presenters` and `uncredited_presenters`. Each carries
`EdgeKind::Owner { target_field: &FIELD_PANELS, exclusive_with: ... }` so the
macro enforces mutual exclusivity on write.

| Panel field                 | Mode       | Semantics                                                                       |
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

## Read-only Computed Fields with Schedule Access

For computed fields that require Schedule access to traverse edges or access other
entities, use `ReadFn::Schedule` with a closure that receives `(&Schedule, EntityId<E>)`:

```rust
// Use callback_field_properties! for computed fields with Schedule access
pub static FIELD_CREDITS: FieldDescriptor<PanelEntityType> = {
    let (data, cb) = callback_field_properties! {
        PanelEntityType,
        name: "credits",
        display: "Credits",
        description: "Formatted presenter credit strings for display.",
        aliases: &["credit"],
        cardinality: List,
        item: String,
        example: "[\"John Doe\", \"Group Name (Alice, Bob)\"]",
        order: 3600,
        read: |sched: &Schedule, id: PanelId| {
            // Access schedule to traverse edges, look up entities, etc.
            // ... compute and return FieldValue
            None
        },
    };
    FieldDescriptor {
        data,
        required: false,
        cb,
    }
};
```

This pattern is used for:

- **Panel**: `credits` (formats credited presenter strings with group resolution),
  `hotel_rooms` (traverses event_rooms to hotel rooms)
- **Presenter**: `inclusive_groups`, `inclusive_members` (transitive closure via
  `inclusive_edges` with explicit `FullEdge` constants)
