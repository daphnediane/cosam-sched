# Panel Entity Field Alignment

## Summary

Align Panel entity fields with schedule-core canonical column definitions.

## Status

Completed

## Priority

High

## Description

Add missing fields to Panel entity and ensure all field aliases include canonical forms from schedule-core for proper field resolution.

## Implementation Details

### Add Missing Fields

- Add `old_uniq_id: Option<String>` field with aliases matching `schedule::OLD_UNIQ_ID` (aliases: "old_uniq_id", "old_uniqid", "old_id", "oldid")
- Add `ticket_sale: Option<String>` field with aliases matching `schedule::TICKET_SALE` (aliases: "ticket_sale", "tickets", "sale")

### Verify and Update Field Aliases

Ensure all field aliases include the canonical form from schedule-core:

- `description`: Add "Description" to aliases
- `prereq`: Add "Prereq" to aliases
- `note`: Add "Note" to aliases
- `notes_non_printing`: Add "Notes_Non_Printing" to aliases
- `workshop_notes`: Add "Workshop_Notes" to aliases
- `power_needs`: Add "Power_Needs" to aliases
- `sewing_machines`: Add "Sewing_Machines" to aliases
- `av_notes`: Add "AV_Notes" to aliases
- `difficulty`: Add "Difficulty" to aliases
- `cost`: Add "Cost" to aliases
- `seats_sold`: Add "Seats_Sold" to aliases
- `pre_reg_max`: Add "Prereg_Max" to aliases
- `capacity`: Add "Capacity" to aliases
- `have_ticket_image`: Add "Have_Ticket_Image" to aliases
- `simple_tix_event`: Add "Simple_Tix_Event" to aliases
- `hide_panelist`: Add "Hide_Panelist" to aliases
- `alt_panelist`: Add "Alt_Panelist" to aliases
- `ticket_url`: Add "Ticket_URL" to aliases
- `is_free`: Add "Is_Free" to aliases
- `is_kids`: Add "Is_Kids" to aliases
- `is_full`: Add "Full" to aliases

## Acceptance Criteria

- Missing fields `old_uniq_id` and `ticket_sale` added to Panel entity
- All field aliases include canonical forms from schedule-core
- Panel entity compiles and passes tests
- Field matching logic resolves canonical names correctly
