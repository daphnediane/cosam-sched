# Query System

## Summary

Implement field-based search, matching, and bulk update operations.

## Status

In Progress

## Priority

Medium

## Description

The query system enables finding and updating entities using field-based
criteria rather than direct UUID access.

### Finder

- `FieldMatch` — criteria struct with field name, operator, and value
- `QueryOptions` — pagination, sorting, field filters
- `find::<T>(matches, options)` → list of matching UUIDs
- `get_many::<T>(matches, options)` → list of matching entity data references

### Matching / Indexing

- `IndexableField<T>` trait for fields that participate in text search
- `MatchPriority` (u8) with standard levels: ExactMatch(255), StrongMatch(200),
  AverageMatch(100), WeakMatch(50), NoMatch(0)
- `FieldMatchResult` with entity UUID, match priority, field priority, field name
- Custom match closures per field (e.g., Panel name with word-boundary matching)

### Updater

- Bulk field updates via field name + FieldValue pairs
- Validation before applying updates
- Integration with edit command system (FEATURE-010) for undo support

### `find_or_create_tagged_presenter` (implemented)

`PresenterEntityType::find_or_create_tagged(storage, input) -> Result<PresenterId, LookupError>`
is the core single-string presenter resolver, living in `entity/presenter.rs`.
`Schedule::find_or_create_tagged_presenter(input)` is a thin delegate to it.

Also exposed through `EntityResolver`: `PresenterEntityType` provides a custom
`impl EntityResolver` whose `resolve_string` delegates to
`find_or_create_tagged` after UUID parsing.

It handles:

| Input form                    | Behavior                                                      |
| ----------------------------- | ------------------------------------------------------------- |
| `presenter-<uuid>`            | typed-ID lookup; `Err(UuidNotFound)` if missing               |
| bare UUID string              | raw UUID parse + lookup; `Err(UuidNotFound)` if missing       |
| `<flags>:[<]NAME[=(=)GROUP]`  | find-or-create with explicit rank from tag                    |
| `[<]NAME[=(=)GROUP]` (no tag) | find-or-create via `process_tagged` with no rank (no upgrade) |
| bare name (no tag, no syntax) | find-or-create with Panelist default; no rank upgrade         |

Tag format:

```text
[<flags>:](<)NAME(==(=)GROUP)
```

- **Flags** (optional prefix before `:`): one or more of `G J I S P F`
  - `G` — `Guest`, `J` — `Judge`, `S` — `Staff`, `I` — `InvitedGuest`,
    `P` — `Panelist` (default), `F` — `FanPanelist`
  - Multiple flags allowed; highest-priority rank wins
- **`<`** (before NAME): `always_grouped = true` on the member
- **`==GROUP`**: add membership edge with `always_shown_in_group = true`
- **`=GROUP`**: add membership edge with default flags
- Group-only form (`==MyGroup` or `G:==MyGroup` where NAME is absent or equals
  GROUP): returns the group's `PresenterId`

Rank handling:

- **Tagged** (`Some(rank)`): upgrades existing presenters when the tag rank
  has higher priority (lower number) than current rank.
- **Untagged / bare** (`None`): creates new presenters with `Panelist` rank
  but never upgrades existing presenters. This preserves ranks like
  `FanPanelist` set by earlier tagged imports.

`LookupError` variants: `Empty`, `UuidNotFound`, `InvalidUuid`, `NameNotFound`
(reserved for IDEA-043 read-only lookup), `UnknownTag`, `OtherSentinel`.

See IDEA-043 for future read-only lookup variants.

### Presenter Tag-String Import (`add_presenters`)

`Schedule::add_presenters(panel_id, tags: &[&str])` parses presenter credit
strings from spreadsheet cells and connects the resulting presenters/groups to a
panel via `PanelToPresenter` edges.

Each tag string is passed through `find_or_create_tagged_presenter` to resolve
or create the presenter, then a `PanelToPresenter` edge is added.

Name-based creation currently uses a direct case-insensitive name scan. When
FEATURE-009 finder is available, it should switch to
`find::<PresenterEntityType>` with `ExactMatch` on the indexable `name` field
for consistency and future index acceleration.

### Panel Computed Fields (in progress)

The `Panel` entity needs computed fields for presenter management:

- `presenters` (read/write) — direct presenters via `PanelToPresenter` edges
- `add_presenters` (write-only) — append presenters without replacing existing
- `remove_presenters` (write-only) — remove specific presenters
- `inclusive_presenters` (read-only) — transitive closure: direct presenters + their
  groups (upward) + members of groups (downward)

Singular aliases: `presenter`, `inclusive_presenter`.

### Presenter Computed Fields (in progress)

The `Presenter` entity needs computed fields for panel and group relationships:

- `panels` (read/write) — direct panels via `PanelToPresenter` edges
- `add_panels` (write-only) — append panels without replacing existing
- `remove_panels` (write-only) — remove specific panels
- `inclusive_panels` (read-only) — all panels this presenter is on, directly or via
  group membership
- `inclusive_members` (read-only) — transitive members if this presenter is a group
- `inclusive_groups` (read-only) — transitive groups this presenter belongs to

Singular aliases: `panel`, `inclusive_panel`, `inclusive_member`, `inclusive_group`.

### PresenterToGroup Rename

The internal edge type `PresenterToGroup` (and generated names
`PresenterToGroupData`, `PresenterToGroupEntityType`, `PresenterToGroupId`,
etc.) should be renamed to `PresenterMembership` (or similar) to better reflect
its role.  This is a pure refactor with no semantic change; defer until
FEATURE-009 is underway to avoid churn while the edge API is still stabilizing.

## Acceptance Criteria

- Can find entities by exact field match
- Can find entities by text search across indexable fields
- Match results are ordered by priority
- Bulk updates apply correctly and validate
- [x] `add_presenters` parses tag strings and creates/connects presenters correctly
- [x] Panel `presenters`, `add_presenters`, `remove_presenters`, `inclusive_presenters` computed fields
- [x] Presenter `panels`, `add_panels`, `remove_panels`, `inclusive_panels` computed fields
- [x] Presenter `inclusive_members`, `inclusive_groups` computed fields
- [x] Singular aliases for all plural computed fields
- Round-trip: tag strings from real schedule data produce the same presenter/group
  graph as a manual edge-by-edge build
- Unit tests for find, match, and update paths

## Related

- [FEATURE-034] Schedule method delegation to entity types — establishes the
  delegation pattern that computed fields use (Schedule -> EntityType -> EdgeType)
