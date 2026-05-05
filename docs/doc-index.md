# Documentation Index

Complete index of cosam-sched documentation, including current documents and placeholders for future documentation.

## Current Documentation

### User Guides

* **cosam-convert.md** - Full CLI reference for `cosam-convert`: all flags, output commands, settings chaining, conflict detection, and examples including the batch-export workflow.

### Core Documentation

* **WORK_ITEMS.md** - Master index of all work items, organized by priority and status. Automatically generated from work-item/ directory.
* **spreadsheet-format.md** - Documentation of the XLSX spreadsheet format used for schedule data import/export.

### Architecture and Design

* **architecture.md** - Overall system architecture, crate layout (schedule-core, schedule-macro), entity/field system overview, Schedule container design, UUID identity, and design decisions.

### Data Model

* **field-system.md** - Entity field system design: three-struct entity pattern, `EntityType` trait, `FieldDescriptor`, `FieldValue`, `CrdtFieldType`, field trait hierarchy, `NamedField::try_as_half_edge()`, `HalfEdgeDescriptor`, global registry module (`get_entity_type`, `get_named_field`, `get_full_edge_by_owner`), `FieldSet`, and error types.
* **conversion-and-lookup.md** - Type-safe conversion system for `FieldValue` to typed Rust outputs, including entity resolution support with `FieldValueForSchedule`, `FieldTypeMapping`, `FieldValueConverter`, and `EntityStringResolver`.

### Storage and Sync

* **crdt-design.md** - CRDT-backed storage design: automerge library choice, `CrdtFieldType` field mappings per entity, merge semantics (LWW / RGA / OR-Set), soft deletes, and phase plan.

### JSON Format Documentation

* **widget-json-format.md** - Standalone widget JSON display format (public-facing format consumed by the calendar widget)

### Work Item System

The work item system tracks all planned and in-progress work for the cosam-sched project.

#### Organization

Work items are stored in `docs/work-item/` as `<PREFIX>-<###>.md` files, automatically organized into subdirectories:

* **meta/** - Project-level meta items and phase trackers (META prefix)
* **high/** - High priority open items
* **medium/** - Medium priority open items
* **low/** - Low priority open items

#### Prefixes

* **META** - Project-level meta items and phase trackers
* **FEATURE** - New functionality
* **BUGFIX** - Fixes for defects
* **UI** - Interface improvements
* **EDITOR** - Desktop editor app
* **CLI** - Command-line interface
* **DEPLOY** - Packaging, deployment, and distribution
* **CLEANUP** - Repository cleanup
* **PERFORMANCE** - Optimizations
* **DOCS** - Documentation
* **REFACTOR** - Code restructuring
* **TEST** - Test additions

#### Workflow

1. Create work item file in `docs/work-item/` with next available number
2. Follow the template structure (Summary, Status, Priority, Description, etc.)
3. Set status and priority appropriately
4. Run `perl scripts/combine-workitems.pl` to organize and regenerate WORK_ITEMS.md

The combine script automatically:

* Moves files to subdirectories based on status/priority
* Generates `docs/WORK_ITEMS.md` with reference-style links
* Adds headerless link glossary at end
* Preserves leading zeros and uses LF line endings

## Placeholder Documentation

The following documents are stubs to be expanded as the project progresses.

### File Formats

* **file-formats.md** - Overview of all file formats used in the project: internal schedule format, XLSX spreadsheet format, JSON widget format, and multi-year archive format. (TODO: Expand with format specifications and conversion workflows)

### Development Guides

* **builders.md** - Entity builder pattern guide for creating and modifying entities. (TODO: Create — see FEATURE-017)

## Documentation Maintenance

When completing work items, update relevant documentation listed in this index. Always cross-reference between documents when changes affect multiple areas.

See `.windsurf/rules/track_work_item.md` or `.claude/rules/track_work_item.md` for the work item tracking workflow and documentation update guidelines.
