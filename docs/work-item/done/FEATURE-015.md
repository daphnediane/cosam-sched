# TimeRange + Panel Entity

## Summary

Port `TimeRange` and implement the Panel entity with stored and computed time fields.

## Status

Completed

## Priority

High

## Blocked By

- FEATURE-013: FieldSet registry

## Description

Panel is the most complex entity with ~25 stored fields plus computed time
projections from `TimeRange` and edge-backed relationship fields.

### TimeRange

Port from v10-try3 (6-variant enum). Setting start, end, or duration in any
order behaves correctly — the canonical pair is always preserved:

- `Unspecified` — no timing information
- `UnspecifiedWithDuration(Duration)` — duration known, no start
- `UnspecifiedWithEnd(NaiveDateTime)` — end known, no start
- `UnspecifiedWithStart(NaiveDateTime)` — start known, no duration or end
- `ScheduledWithDuration { start_time, duration }` — start + duration canonical; end computed
- `ScheduledWithEnd { start_time, end_time_minus_start_time: Duration }` — start + end canonical; the delta is stored internally (not `end_time` directly) so the variant cannot represent an inverted range. Consumers still treat `end_time` as canonical: mutations that change `start_time` adjust the duration to keep the same end.

### Three structs

**`PanelCommonData`** (`pub`) — hand-written, serializable, user-facing fields
from the **Schedule** sheet (~24 fields):

- `uid: String` — raw Uniq ID string (required, indexed)
- `name: String` — panel title (required, indexed)
- `description: Option<String>` — attendee-facing description (Text/CRDT)
- `note: Option<String>`
- `notes_non_printing: Option<String>`
- `workshop_notes: Option<String>`
- `power_needs: Option<String>`
- `sewing_machines: bool`
- `av_notes: Option<String>`
- `difficulty: Option<String>`
- `prereq: Option<String>`
- `cost: Option<String>` — raw cost cell value (e.g. `"$35"`, `"Kids"`)
- `is_free: bool` — parsed from cost during import
- `is_kids: bool` — parsed from cost during import
- `is_full: bool`
- `capacity: Option<i64>`
- `seats_sold: Option<i64>`
- `pre_reg_max: Option<i64>`
- `ticket_url: Option<String>`
- `have_ticket_image: bool`
- `simpletix_event: Option<String>` — internal admin URL for ticket configuration
- `simpletix_link: Option<String>` — public-facing direct ticket purchase link
- `hide_panelist: bool`
- `alt_panelist: Option<String>`

**`PanelInternalData`** (`pub(crate)`) — `EntityType::InternalData`; the field system operates on this:

- `data: PanelCommonData`
- `code: PanelId` — typed UUID identity
- `time_slot: TimeRange` — canonical timing; exposed via computed time fields
- `parsed_uid: Option<PanelUniqId>` — parsed Uniq ID components, set during import

**`PanelData`** (`pub`) — export/API view, produced by `export(&Schedule)`:

- `data: PanelCommonData`
- `code: String` — stringified `PanelId`
- `start_time: Option<NaiveDateTime>` — projected from `time_slot`
- `end_time: Option<NaiveDateTime>` — projected from `time_slot`
- `duration: Option<Duration>` — projected from `time_slot`
- `presenter_ids: Vec<PresenterId>` — assembled from edge maps
- `event_room_ids: Vec<EventRoomId>` — assembled from edge maps (panels can occupy multiple rooms)
- `panel_type_id: Option<PanelTypeId>` — assembled from edge maps; singular (panels currently have one type)

Design note: a panel's type is conventionally derived from its Uniq ID prefix
(e.g. prefix `"GW"` → workshop-type panels). Currently modeled as
`Option<PanelTypeId>` — one type per panel. Multiple-type support (e.g.
distinguishing GW/FW/WS as separate workshop sub-types) is deferred; open
a new work item if that becomes needed.

### Field descriptors

Closures access `internal.data.*` for `CommonData` fields, `internal.time_slot`
for time projections, and `internal.code` for the ID field.

`export()` takes `&Schedule` directly. The `&dyn FieldDatabase` abstraction
seen in v10-try3 was addressing a layer violation in progress; revisit only
if concrete layer separation needs arise.

Edge-backed relationship computed fields (read/write stubs; full implementation
in FEATURE-018):

- `presenters`, `add_presenters`, `remove_presenters`, `inclusive_presenters`
- `event_rooms` (alias `rooms`), `add_rooms`, `remove_rooms`
- `panel_type` (alias `kind`) — read/write singular `Option<PanelTypeId>`

## Acceptance Criteria

- TimeRange serialization round-trips correctly
- PanelData compiles with all stored fields
- Time projection computed fields read correctly
- Unit tests for TimeRange operations and field read/write
- Serialization round-trip test for PanelData
