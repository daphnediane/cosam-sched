# Documentation Index

Complete index of cosam-sched documentation, including current documents and placeholders for future documentation.

## Current Documentation

### Core Documentation

* **WORK_ITEMS.md** - Master index of all work items, organized by priority and status. Automatically generated from work-item/ directory.
* **spreadsheet-format.md** - Documentation of the XLSX spreadsheet format used for schedule data import/export.

### JSON Format Documentation

#### Top-level JSON Docs

* **json-format-v4.md** - JSON format version 4 specification
* **json-private-v5.md** - Private JSON format version 5 specification
* **json-private-v6.md** - Private JSON format version 6 specification
* **json-public-v5.md** - Public JSON format version 5 specification
* **json-public-v6.md** - Public JSON format version 6 specification
* **json-v7-display.md** - JSON version 7 display format
* **json-v7-full.md** - JSON version 7 full format
* **json-v8-full.md** - JSON version 8 full format
* **json-v9-display.md** - JSON version 9 display format
* **json-v9-full.md** - JSON version 9 full format
* **json-v10-display.md** - JSON version 10 display format
* **json-v10-full.md** - JSON version 10 full format

#### JSON Schema Documentation (json-schedule/)

The `json-schedule/` directory contains detailed schema documentation for the calendar widget display format:

* **README.md** - Overview of the JSON schema documentation
* **v4.md, v5-*.md, v6-*.md, v7-*.md, v8.md, v9-*.md, v10-*.md** - Version-specific schema documentation
* **Panel-v9.md, PanelPart-v5.md, PanelSession-v5.md** - Entity-specific schemas
* **presenters-*.md, rooms-*.md, panelTypes-*.md** - Domain entity schemas
* **meta-*.md, timeline-*.md, conflicts-*.md** - Metadata and relationship schemas

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

The following documents are placeholders to be expanded as the project progresses:

### Architecture and Design

* **architecture.md** - Overall system architecture, crate layout, and design decisions. (TODO: Expand with current system architecture)

### Data Model

* **field-system.md** - Entity field system design, including the `#[derive(EntityFields)]` macro, field traits, FieldValue enum, and validation infrastructure. (TODO: Expand with current field system implementation)

### Storage and Sync

* **crdt-design.md** - CRDT-backed storage design for offline collaborative editing, including field type mappings and merge semantics. (TODO: Expand with settled CRDT design decisions)

### File Formats

* **file-formats.md** - Overview of all file formats used in the project: internal schedule format, XLSX spreadsheet format, JSON widget format, and multi-year archive format. (TODO: Expand with format specifications and conversion workflows)

### Development Guides

* **builders.md** - Entity builder pattern guide for creating and modifying entities. (TODO: Create)

## Documentation Maintenance

When completing work items, update relevant documentation listed in this index. Always cross-reference between documents when changes affect multiple areas.

See `.windsurf/rules/track_work_item.md` for the work item tracking workflow and documentation update guidelines.
