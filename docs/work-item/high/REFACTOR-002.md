# Field Alignment with Schedule-Core Canonical Columns

## Summary

Align all entity fields with spreadsheet canonical columns and schedule-core equivalents.

## Status

Not Started

## Priority

High

## Description

Review and align entity fields with `crates/schedule-core/src/xlsx/columns.rs` canonical column definitions. Ensure field names and aliases match `docs/spreadsheet-format.md` and JSON format specifications. Add any missing canonical fields.

## Implementation Details

- Panel: verify coverage of `description`, `prereq`, `note`, `notes_non_printing`, `workshop_notes`, `power_needs`, `sewing_machines`, `av_notes`, `difficulty`, `cost`, `seats_sold`, `pre_reg_max`, `capacity`, `have_ticket_image`, `simple_tix_event`, `ticket_url`, `hide_panelist`, `alt_panelist`, `is_free`, `is_kids`, `is_full`
- EventRoom: align with room_map fields (`sort_key`, `long_name`, `hotel_room`)
- HotelRoom: ensure proper hotel room mapping and sort key
- PanelType: align with panel_types fields (`panel_kind`, `color`, `bw_color`, `hidden`, `is_timeline`, `is_private`, `is_break`, `is_workshop`, `is_room_hours`, `is_cafe`, `is_virtual`)
- Presenter: align with people fields (`classification`, `is_group`, `members`, `groups`, `always_grouped`, `always_shown`)
- Verify all field aliases match spreadsheet column headers and JSON camelCase equivalents
- Add `#[field(alias = ...)]` attributes where needed

## Acceptance Criteria

- Every canonical spreadsheet column has a corresponding entity field
- Field aliases resolve correctly for both spreadsheet headers and JSON keys
- No missing fields compared to `schedule-core` column definitions
- All entities compile and pass tests after alignment
