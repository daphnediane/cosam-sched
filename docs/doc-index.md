# Documentation Index

Complete index of cosam-sched documentation, including current documents and placeholders for future documentation.

## Current Documentation

### User Guides

* **cosam-convert.md** - Full CLI reference for `cosam-convert`: all flags, output commands, settings chaining, conflict detection, and examples including the batch-export workflow.

### Core Documentation

* **WORK_ITEMS.md** - Master index of all work items, organized by priority and status. Automatically generated from work-item/ directory.
* **work-item-system.md** - Documentation of the work item system: organization, prefixes, workflow, and automation.
* **spreadsheet-format.md** - Documentation of the XLSX spreadsheet format used for schedule data import/export.

### Architecture and Design

* **architecture.md** - Overall system architecture, crate layout (schedule-core, schedule-macro), entity/field system overview, Schedule container design, UUID identity, and design decisions.
* **layout-formats.md** - Print layout formats (`schedule-layout` crate): the `generate` contract and filename conventions, shared building blocks (preamble, banner, grid, panel blocks, the `place`+`colbreak` grid/column mixing), each format (schedule, descriptions, workshops, room signs, flyer, guest postcards), and per-paper column counts.

### Data Model

* **field-system.md** - Entity field system design: three-struct entity pattern, `EntityType` trait, `FieldDescriptor`, `FieldValue`, `CrdtFieldType`, field trait hierarchy, `NamedField::try_as_half_edge()`, `HalfEdgeDescriptor`, global registry module (`get_entity_type`, `get_named_field`, `get_full_edge_by_owner`), `FieldSet`, and error types.
* **conversion-and-lookup.md** - Type-safe conversion system for `FieldValue` to typed Rust outputs, including entity resolution support with `FieldValueForSchedule`, `FieldTypeMapping`, `FieldValueConverter`, and `EntityStringResolver`.

### Storage and Sync

* **crdt-design.md** - CRDT-backed storage design: automerge library choice, `CrdtFieldType` field mappings per entity, merge semantics (LWW / RGA / OR-Set), `__extra` CRDT map for unknown columns, `ScheduleSidecar` (ephemeral per-session data), `ChangeState` tracking, and save/load semantics.
* **field-comparison.md** - Cross-version field comparison: which XLSX columns exist in v9, v10-try1, v10-try3, and main; year-by-year spreadsheet column tables (2016–2026); gaps in both directions.

### JSON Format Documentation

* **widget-json-format.md** - Standalone widget JSON display format (public-facing format consumed by the calendar widget)
* **widget-config-format.md** - Presentation configuration format (branding and print formats) that can be loaded independently from schedule data
* **widget-html-format.md** - Hybrid widget-html embedded format: structural JSON block (meta, rooms, panelTypes, timeline, presenters) plus semantic HTML panel elements for search engine visibility and no-JS progressive enhancement

## Placeholder Documentation

The following documents are stubs to be expanded as the project progresses.

### File Formats

* **file-formats.md** - Overview of all file formats used in the project: internal schedule format, XLSX spreadsheet format, JSON widget format, and multi-year archive format. (TODO: Expand with format specifications and conversion workflows)

### Development Guides

* **builders.md** - Entity builder pattern guide for creating and modifying entities. (TODO: Create — see FEATURE-017)
