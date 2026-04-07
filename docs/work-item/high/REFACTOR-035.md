# PanelType Entity Field Alignment

## Summary

Align PanelType entity field aliases with schedule-core canonical column definitions.

## Status

Not Started

## Priority

High

## Description

Ensure PanelType entity field aliases include canonical forms from schedule-core for proper field resolution, and fix duplicate alias.

## Implementation Details

### Verify and Update Field Aliases

- `prefix`: Add canonical "Prefix" to aliases
- `kind`: Add canonical "Panel_Kind" to aliases
- `color`: Add canonical "Color" to aliases
- `bw_color`: Add canonical "BW" to aliases (remove duplicate "bw_color")
- `is_hidden`: Add canonical "Hidden" to aliases
- `is_timeline`: Add canonical "Is_Timeline" to aliases (include "Is_Time_Line" for old format compatibility)
- `is_private`: Add canonical "Is_Private" to aliases
- `is_break`: Add canonical "Is_Break" to aliases
- `is_workshop`: Add canonical "Is_Workshop" to aliases
- `is_room_hours`: Add canonical "Is_Room_Hours" to aliases
- `is_cafe`: Add canonical "Is_Café" to aliases (include "Is_Cafe" variant)

## Acceptance Criteria

- All field aliases include canonical forms from schedule-core
- Duplicate alias removed from bw_color field
- PanelType entity compiles and passes tests
