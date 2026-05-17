# REFACTOR-125: Consolidate ImportContext to carry import state

## Summary

Move schedule, options, per-pass lookups, and PresenterImportCache into
ImportContext; convert reader free functions to methods on ImportContext.

## Status

In Progress

## Priority

Medium

## Description

`ImportContext` currently holds only the read-side context (book, file_path,
import_time, csv_map). Schedule, per-table import modes (via options), and
inter-stage lookups (panel_type_lookup, room_lookup, hotel_lookup) are passed
as separate parameters to every reader function, creating long signatures and
redundant plumbing.

This refactor:

- Adds `schedule`, `options`, `presenter_cache`, `panel_type_lookup`,
  `room_lookup`, and `hotel_lookup` as fields of `ImportContext`.
- Changes `find_data_range` / `find_table` / `find_sheet` to take
  `(book, csv_map, mode, names)` directly, removing the ctx dependency
  (only book + csv_map are needed) and eliminating the borrow conflict when
  reading `ctx.options.<table>` alongside `&mut ctx`.
- Converts each `read_<table>_into` free function to a `pub(super)` method
  on `ImportContext` via `impl ImportContext<'_>` blocks in the respective
  module files.
- Simplifies `update_schedule_from_xlsx` to create the context once and call
  `ctx.read_panel_types()`, `ctx.read_hotel_rooms()`, etc.
- `collect_presenters` in schedule.rs retains separate field params because it
  is called while `ws` (borrowed from `ctx.book`) is alive.
