# Conversion and Lookup System

## Overview

Two cooperating systems:

- **Lookup** (`schedule-core::lookup`) — query strings → `EntityId<E>` via
  entity-owned match logic, with multi-token splitting, UUID fast-paths,
  cardinality enforcement, and optional find-or-create.
- **Conversion** — type-safe conversion between `FieldValue` inputs and
  typed Rust outputs, layered as:
  1. **FieldTypeMapping** — type-safe conversions without schedule context
  2. **EntityStringResolver** — entity string resolution with schedule context
     (currently built on top of the lookup system; pending refactor)
  3. **FieldValueConverter** — custom conversion logic with schedule context

## Lookup System

Entity-level text matching. Replaces the earlier per-field
`IndexableField<E>` / `index_fn` / `FieldSet::match_index()` machinery.

### MatchPriority

`MatchPriority` is a `u8` score (`0` = no match, `255` = exact). Use the
named constants in `match_priority`:

| Constant        | Value | Meaning                               |
| --------------- | ----- | ------------------------------------- |
| `NO_MATCH`      | 0     | No match (equivalent to `None`)       |
| `MIN_MATCH`     | 1     | Minimum acceptable match level        |
| `WEAK_MATCH`    | 50    | Substring anywhere inside value       |
| `AVERAGE_MATCH` | 100   | Match at a word boundary              |
| `STRONG_MATCH`  | 200   | Value starts with query               |
| `EXACT_MATCH`   | 255   | Value equals query (case-insensitive) |

The helper `string_match_priority(query, value) -> Option<MatchPriority>`
implements the standard tiered match and is the normal building block for
`EntityMatcher::match_entity` implementations.

### EntityMatcher

```rust
pub trait EntityMatcher: EntityType {
    fn match_entity(query: &str, data: &Self::InternalData) -> Option<MatchPriority>;
}
```

Each entity type owns its full matching logic, combining any fields it
chooses (e.g. `PanelEntityType` matches on both `code` and `name`;
`EventRoomEntityType` on `room_name` and `long_name`). This replaces the
old per-field indexing where each `FieldDescriptor` carried an `index_fn`.

### EntityCreatable

```rust
pub enum CanCreate { No, FromFull, FromPartial }

pub trait EntityCreatable: EntityMatcher {
    fn can_create(full: &str, partial: &str) -> CanCreate;
    fn create_from_string(schedule: &mut Schedule, s: &str)
        -> Result<EntityId<Self>, LookupError>;
}
```

Implemented by `PresenterEntityType`, `EventRoomEntityType`,
`HotelRoomEntityType`, and `PanelTypeEntityType`. Not implemented by
`PanelEntityType` (panels are never auto-created by lookup).

### Lookup API

All functions are free functions in `schedule-core::lookup`. Cardinality is
expressed via [`FieldCardinality`] (`Single` / `Optional` / `List`).
Results are returned as `Vec<EntityId<E>>`; `Single` yields exactly one,
`Optional` yields zero or one, `List` yields any number.

```rust
pub fn lookup<E: EntityMatcher>(
    schedule: &Schedule,
    query: &str,
    cardinality: FieldCardinality,
) -> Result<Vec<EntityId<E>>, LookupError>;

pub fn lookup_or_create<E: EntityCreatable>(
    schedule: &mut Schedule,
    query: &str,
    cardinality: FieldCardinality,
) -> Result<Vec<EntityId<E>>, LookupError>;

// Convenience helpers that specialize the common cardinalities:
pub fn lookup_single<E: EntityMatcher>(schedule: &Schedule, query: &str)
    -> Result<EntityId<E>, LookupError>;
pub fn lookup_list<E: EntityMatcher>(schedule: &Schedule, query: &str)
    -> Result<Vec<EntityId<E>>, LookupError>;
pub fn lookup_or_create_single<E: EntityCreatable>(
    schedule: &mut Schedule, query: &str,
) -> Result<EntityId<E>, LookupError>;
pub fn lookup_or_create_list<E: EntityCreatable>(
    schedule: &mut Schedule, query: &str,
) -> Result<Vec<EntityId<E>>, LookupError>;
```

Optional cardinality is available via the non-`_single`/`_list` functions
with `FieldCardinality::Optional`.

### Algorithm sketch

- Trim query, then loop:
  1. If more than one result has been accumulated and cardinality is
     limited (`Single`/`Optional`) → `TooMany`.
  2. Split remaining query at first `,` or `;` into `(partial, rest)`.
  3. If the partial token looks like a bare UUID or `"type_name:<uuid>"`
     tagged UUID, fast-path via `parse_typed_uuid::<E>()` (validates entity
     type and existence) and continue with `rest`.
  4. Otherwise scan all entities of type `E` against the full remaining
     query and against the partial token, keeping matches at the best
     priority. Ties prefer the full-string match.
  5. Zero matches → `NotFound`, or for `lookup_or_create` consult
     `E::can_create(full, partial)` and push onto a deferred create queue.
  6. Exactly one match → record it; advance by rest or clear (depending
     on which variant matched).
  7. More than one at the best priority → `AmbiguousMatch`.
- After the loop, check cardinality once more on `results + create_queue`
  before running deferred creates, then run them in order and return.

### LookupError

```rust
pub enum LookupError {
    AmbiguousMatch  { query: String },
    NotFound        { query: String },
    WrongEntityType { expected: &'static str, got: String },
    TooMany         { found: usize },
    CannotCreate    { query: String },
    InvalidUuid     { s: String },
    CreateFailed    { message: String },
}
```

## FieldTypeMapping

Marker types that map `FieldValueItem` variants to Rust types:

- `AsString` → `String`
- `AsText` → `String`
- `AsInteger` → `i64`
- `AsFloat` → `f64`
- `AsBoolean` → `bool`
- `AsDateTime` → `chrono::NaiveDateTime`
- `AsDuration` → `chrono::Duration`
- `AsEntityId<E>` → `EntityId<E>`

### Cross-Type Conversions

All marker types support cross-type conversions:

**AsString** converts from:

- `Integer` → string representation
- `Float` → string representation
- `Boolean` → "true" or "false"
- `DateTime` → formatted as "Day HH:MM XM"
- `Duration` → "H:MM" or "MM"
- `EntityIdentifier` → entity-specific string (name, code, etc.)

**AsInteger** converts from:

- `String` → parsed integer
- `Float` → integer if whole number
- `Duration` → minutes

**AsFloat** converts from:

- `String` → parsed float
- `Integer` → float
- `Duration` → minutes

**AsBoolean** converts from:

- `String` → "true"/"false"/"yes"/"no"/"1"/"0"
- `Integer` → true if non-zero
- `Float` → true if non-zero

**AsDateTime** converts from:

- `String` → ISO-8601 format parsing

**AsDuration** converts from:

- `String` → "HH:MM" or minutes parsing
- `Integer` → minutes
- `Float` → minutes

**`AsEntityId<E>`** converts from:

- `EntityIdentifier` → validates type match
- `String` → requires schedule context (use FieldValueConverter)

## EntityStringResolver

Trait for entity types to provide custom string resolution:

```rust
pub trait EntityStringResolver: EntityType {
}
```

### FieldValueConverter

Converts individual `FieldValueItem` values with optional entity resolution.

```rust
pub trait FieldValueConverter<M: FieldTypeMapping> {
    fn lookup_next(&self, schedule: &Schedule, input: FieldValueItem) 
        -> Option<Result<M::Output, ConversionError>>;
    
    fn lookup_or_create_next(&self, schedule: &mut Schedule, input: FieldValueItem) 
        -> Option<Result<M::Output, ConversionError>>;
    
    fn select_one(&self, outputs: Vec<M::Output>) -> Result<Option<M::Output>, ConversionError>;
}
```

### EntityStringResolver Trait

Entity types implement this trait to provide custom string resolution and formatting:

```rust
pub trait EntityStringResolver: EntityType {
    // String -> EntityId (lookup)
    fn lookup_string(schedule: &Schedule, s: &str) -> Option<EntityId<Self>>;
    fn lookup_or_create_string(schedule: &mut Schedule, s: &str) -> Result<EntityId<Self>, ConversionError>;
    fn lookup_string_many(schedule: &Schedule, s: &str) -> Vec<EntityId<Self>>;
    
    // EntityId -> String (formatting)
    fn entity_to_string(schedule: &Schedule, id: EntityId<Self>) -> String;
    fn entity_to_string_many(schedule: &Schedule, ids: Vec<EntityId<Self>>) -> String;
}
```

## FieldValueForSchedule

Wrapper for `FieldValue` with schedule context that provides explicit mode selection:

```rust
pub enum FieldValueForSchedule<'a> {
    Lookup(&'a Schedule, FieldValue),           // Read-only lookup
    LookupOrCreate(&'a mut Schedule, FieldValue), // Create-or-resolve
}

impl<'a> FieldValueForSchedule<'a> {
    pub fn into<M: FieldTypeMapping, C: FieldValueConverter<M>>(
        self,
        converter: &C,
    ) -> Result<M::Output, ConversionError>;
}
```

## Conversion APIs

### Simple Scalar Conversions (No Schedule Needed)

Use `into_...` methods on `FieldValue` and `FieldValueItem` for simple scalar conversions:

```rust
let integer: i64 = field_value.into_integer()?;
let float: f64 = field_value.into_float()?;
let boolean: bool = field_value.into_bool()?;
let datetime: NaiveDateTime = field_value.into_datetime()?;
let duration: Duration = field_value.into_duration()?;
```

### Simple String Conversion (No Entity Formatting)

```rust
let string: String = field_value.into_string()?;
let text: String = field_value.into_text()?;
```

Note: This is simple string-to-string conversion. For entity-specific formatting (e.g., panel code:name), use `EntityStringResolver::entity_to_string` instead.

### Entity Resolution (Requires Schedule Context)

Use `FieldValueForSchedule` for entity resolution:

```rust
// Read-only lookup
let id: EntityId<<Presenter> = FieldValueForSchedule::Lookup(&schedule, field_value!("P:John Smith"))
    .into(&converter)?;

// Create-or-resolve
let id: EntityId<<Presenter> = FieldValueForSchedule::LookupOrCreate(&mut schedule, field_value!("P:NewPresenter"))
    .into(&converter)?;
```

### Entity-Specific String Formatting

Use `EntityStringResolver::entity_to_string` for EntityId -> String conversion with entity-specific formatting:

```rust
// Panels: "<code>: <name>" (e.g., "GP: Cosplay Foam Armor 101")
// Presenters: name (e.g., "John Smith")
// Event rooms and hotel rooms: room name (e.g., "Ballroom East")

let formatted: String = PresenterEntityType::entity_to_string(&schedule, entity_id);
let comma_separated: String = PresenterEntityType::entity_to_string_many(&schedule, vec![id1, id2]);
```

## Driver Functions

Six driver functions expand `FieldValue::List` as a work queue:

### Read-only conversions (lookup)

- `lookup_one<M, C>(converter: &C, schedule: &Schedule, input: FieldValue)` - Returns first successful conversion
- `lookup_optional<M, C>(converter: &C, schedule: &Schedule, input: FieldValue)` - Returns None for empty input
- `lookup_many<M, C>(converter: &C, schedule: &Schedule, input: FieldValue)` - Returns all successful conversions

### Mutable conversions (resolve)

- `resolve_one<M, C>(converter: &C, schedule: &mut Schedule, input: FieldValue)` - Returns first successful conversion
- `resolve_optional<M, C>(converter: &C, schedule: &mut Schedule, input: FieldValue)` - Returns None for empty input
- `resolve_many<M, C>(converter: &C, schedule: &mut Schedule, input: FieldValue)` - Returns all successful conversions

## Standard Marker Types

### Scalar Types

- `AsString` - String conversion
- `AsText` - Text conversion (same as AsString for now)
- `AsInteger` - Integer conversion
- `AsFloat` - Float conversion
- `AsBoolean` - Boolean conversion
- `AsDateTime` - DateTime conversion
- `AsDuration` - Duration conversion

### Entity Types

- `AsEntityId<E>` - EntityId conversion with `EntityStringResolver` support

## Cross-Type Conversion Rules

### AsString

- From String: direct
- From Integer: decimal string
- From Float: decimal string
- From Boolean: "true"/"false"
- From DateTime: ISO 8601 string
- From Duration: "HH:MM" format
- From EntityIdentifier: UUID string

### AsInteger

- From Integer: direct
- From String: decimal parsing
- From Float: whole number only
- From Duration: minutes

### AsFloat

- From Float: direct
- From String: decimal parsing
- From Integer: cast
- From Duration: minutes

### AsBoolean

- From Boolean: direct
- From String: "true"/"false" (case-insensitive)
- From Integer: non-zero = true
- From Float: non-zero = true

### AsDateTime

- From DateTime: direct
- From String: ISO 8601 parsing (US format for MM/DD/YYYY)

### AsDuration

- From Duration: direct
- From String: "HH:MM" or minutes
- From Integer: minutes
- From Float: minutes

## Design Notes

### Explicit Mode Selection

`FieldValueForSchedule` provides explicit mode selection (Lookup vs LookupOrCreate) to prevent bugs where the wrong conversion mode is used. The enum variants enforce at compile-time whether you're doing a read-only lookup or a create-or-resolve operation.

### Work Queue Iteration

The driver functions use a work queue pattern to handle `FieldValue::List` inputs. Each item in the list is processed through the converter's `lookup_next` or `lookup_or_create_next` method, and results are collected. The `select_one` method allows custom selection logic (e.g., highest rank, most recent).

### Read-Only vs Create-or-Resolve

- `lookup_next` is for read-only operations (entity lookups that don't modify the schedule)
- `lookup_or_create_next` is for mutable operations that may create new entities
- This separation prevents accidental entity creation during read operations

### Entity Resolution Context

Entity resolution requires schedule context because:

1. Entity lookups need access to the schedule's entity storage
2. Entity creation may modify the schedule's entity storage
3. Custom lookup logic may depend on schedule state

The `into_...` methods on `FieldValue` and `FieldValueItem` support simple scalar conversions but do NOT support entity resolution with schedule context. Use `FieldValueForSchedule` for entity resolution.

### Entity-Specific String Formatting Implementation

Entity types can customize their string representation by implementing `entity_to_string`:

- Panels: `<code>: <name>` format for easy identification
- Presenters: just the name
- Event rooms and hotel rooms: room name

This allows consistent display formatting across the application while maintaining type safety.

## EntityType Integration

Entity types implement `EntityStringResolver` for custom resolution. The trait
provides a default `lookup_by_uuid_string` that handles both bare UUID strings
and `type_name-<uuid>` prefixed strings, checking entity existence in the schedule.

All five entity types implement at minimum `entity_to_string` (returns the
human-readable name). Types that support find-or-create also override
`lookup_string` and `lookup_or_create_string`.

### Presenter tagged credit strings

`PresenterEntityType` overrides both `lookup_string` and `lookup_or_create_string`
to support the full tagged credit-string format:

```
[Kind:][ < ]Name[ = [ = ]Group]
```

| Form              | Meaning                                                         |
| ----------------- | --------------------------------------------------------------- |
| `P:Alice`         | Alice with Panelist rank                                        |
| `G:Alice`         | Alice with Guest rank                                           |
| `Alice=MyBand`    | Alice in group MyBand                                           |
| `P:Alice==MyBand` | Alice in MyBand; group sets `always_shown_in_group`             |
| `P:<Alice=MyBand` | Alice (always_grouped) in MyBand                                |
| `==MyBand`        | Group-only: create/find MyBand as explicit group (always_shown) |
| `=MyBand`         | Group-only: find group named MyBand                             |
| `P:==MyBand`      | Create MyBand as explicit always-shown group with Panelist rank |

- **Kind prefix**: one or more chars from `G/J/S/I/P/F`; highest-priority (lowest
  number) rank among them is applied.
- **Rank upgrade**: existing presenters are upgraded when the requested rank is
  higher (lower priority number); ranks are never downgraded; bare names (no `Kind:`)
  never change an existing presenter's rank.
- **Group detection** (`=Group` and group-only forms): a presenter is treated as
  a group if `is_explicit_group` is set **or** it has at least one member edge.

```rust
impl EntityStringResolver for PresenterEntityType {
    fn lookup_string(schedule: &Schedule, s: &str) -> Option<EntityId<Self>> {
        Self::lookup_by_uuid_string(schedule, s)
            .or_else(|| find_tagged_presenter(schedule, s))
    }

    fn lookup_or_create_string(schedule: &mut Schedule, s: &str)
        -> Result<EntityId<Self>, ConversionError>
    {
        if let Some(id) = Self::lookup_by_uuid_string(schedule, s) {
            return Ok(id);
        }
        find_or_create_tagged_presenter(schedule, s)
    }
}
```

### Other entity types

`HotelRoomEntityType` and `EventRoomEntityType` also override
`lookup_or_create_string` to find-or-create by room name using a deterministic
v5 UUID derived from the name.

`PanelEntityType` and `PanelTypeEntityType` use the default implementation
(UUID lookup then `EntityMatcher::match_entity` scan; `PanelType` additionally
implements `EntityCreatable` for find-or-create, while `Panel` is never
auto-created).
