# Work Item System

The work item system tracks all planned and in-progress work for the cosam-sched project.

## Organization

Work items are stored in `work-item/` (project root) as `<PREFIX>-<###>.md` files, automatically organized into subdirectories. The script automatically creates all required subdirectories if they don't exist.

### File Naming Convention

* Format: `<PREFIX>-<###>.md` (e.g., `FEATURE-001.md`, `BUGFIX-023.md`)
* Prefixes are uppercase (e.g., FEATURE, BUGFIX, META, IDEA)
* Numbers are zero-padded to three digits (001, 002, etc.)
* The script automatically assigns the next available ID number when creating new items
* Leading zeros are preserved in filenames and generated documentation

* **open/1-HIGH/** - High priority open items
* **open/2-MEDIUM/** - Medium priority open items
* **open/3-LOW/** - Low priority open items
* **open/4-NEW/** - Placeholder stubs awaiting details
* **idea/** - IDEA prefix items
* **meta/** - Project-level meta items and phase trackers (META prefix)
* **closed/done/** - Completed items
* **closed/rejected/** - Rejected items
* **closed/superseded/** - Superseded items
* **template/** - Per-prefix and default templates

## Prefixes

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

## Statuses

Work items can have the following statuses. The script accepts both canonical values and common aliases.

* **Placeholder** - Initial stub state for newly created items. Edit to `Open` when ready to begin work.
  * Aliases: stub, template
* **Open** - Item is actively being worked on or is ready to be started.
  * Aliases: new, todo, "not started"
* **In Progress** - Work is currently underway on this item.
  * Aliases: started, working
* **Blocked** - Work cannot proceed due to dependencies or external factors. Use `Blocked By` section to specify blockers.
  * Aliases: waiting
* **Completed** - Item has been completed and verified. Items with this status are moved to `closed/done/`.
  * Aliases: done, finished, complete, closed
* **Rejected** - Item was decided against and will not be implemented. Items with this status are moved to `closed/rejected/`.
  * Aliases: declined, wontfix
* **Superseded** - Item has been replaced by a different approach or implementation. Items with this status are moved to `closed/superseded/`.
  * Aliases: replaced

## Priorities

Work items can have the following priorities. The script accepts both canonical values and common aliases.

* **High** - Urgent items that should be addressed soon. Items with this priority are placed in `open/1-HIGH/`.
  * Aliases: hi, critical, urgent, raise
* **Medium** - Standard priority for most work items. Items with this priority are placed in `open/2-MEDIUM/`.
  * Aliases: mid, med, normal, default
* **Low** - Nice-to-have items that can be deferred. Items with this priority are placed in `open/3-LOW/`.
  * Aliases: minor

## Work Item Fields

### Required Sections

All work item files must contain the following sections:

* **Title** - The first line heading (`# PREFIX-###: Brief title`)
* **Summary** - One-line summary of the work item (required for parsing)
* **Status** - Current status of the work item (required for parsing)
* **Priority** - Priority level (required for parsing)
* **Description** - Detailed description (required for parsing)

### Optional Sections

* **Blocked By** - List other work items that must be completed before this one can start. Use the format `- PREFIX-###: short description` to specify dependencies. When a work item is blocked, its status should be set to `Blocked`. The script extracts just the ID from each bullet line for dependency tracking.
* **Work Items** (META-specific) - For META items, this field lists the specific work items that fall under this meta-item or phase. Use the format `- PREFIX-###: short description` to track related work.

### Summary

A one-line summary of the work item. This should be concise and descriptive, allowing readers to quickly understand the item's purpose. This field is required for parsing by the work-item-update.pl script.

### Description

A detailed description of the work item. The content varies by prefix:

* **FEATURE** - Describe the new functionality, its purpose, and implementation approach
* **BUGFIX** - Describe the bug, its impact, and the fix approach
* **UI** - Describe the interface improvement and user experience goals
* **EDITOR/CLI/DEPLOY** - Describe the specific changes and their rationale
* **CLEANUP/PERFORMANCE/REFACTOR** - Describe what will be cleaned up, optimized, or restructured
* **DOCS** - Describe what documentation needs to be created or updated
* **TEST** - Describe what test coverage needs to be added
* **IDEA** - Include motivation, alternatives considered, and open questions
* **META** - Describe the phase, milestone, or project-level initiative

### Additional Sections (optional)

Work items may include additional sections as needed:

* **Steps to Fix** (BUGFIX) - Description of the fix approach
* **How Found** (BUGFIX) - How the bug was discovered (manual testing, user report, CI failure, code review)
* **Reproduction** (BUGFIX) - Steps to reproduce the bug with expected vs actual behavior
* **Testing** (BUGFIX) - How to verify the fix (manual steps, test cases added, regression coverage)
* **Implementation Details** (FEATURE) - Technical implementation details and design decisions
* **Acceptance Criteria** (FEATURE) - Specific criteria that must be met for the item to be considered done
* **Notes** - Additional notes, context, or related information

## Workflow

1. Create work item file with `perl scripts/work-item-update.pl --create <PREFIX>`
   * The script automatically assigns the next available ID number for that prefix
   * Multiple prefixes can be created at once: `--create FEATURE,BUGFIX` or `--create FEATURE --create BUGFIX`
   * Creates the file in the appropriate template directory using the prefix-specific template if available
2. Fill in the stub: edit status from `Placeholder` to `Open`, add description
3. Set priority appropriately
4. Run `perl scripts/work-item-update.pl` to organize and regenerate docs/WORK_ITEMS.md

The combine script automatically:

* Moves files to subdirectories based on status/priority
* Generates `docs/WORK_ITEMS.md` with reference-style links
* Generates `docs/FUTURE_IDEAS.md` for IDEA prefix items
* Adds headerless link glossary at end
* Preserves leading zeros and uses LF line endings

## Sorting and Display

Work items are sorted in the generated WORK_ITEMS.md as follows:

1. **META items** appear first (as phase trackers)
2. **Within META items**: sorted by prefix then number
3. **Non-META items**: sorted by priority (High → Medium → Low), then by prefix, then by number
4. **IDEA items** are excluded from WORK_ITEMS.md and appear in FUTURE_IDEAS.md instead

## META Item Relationships

META items serve as phase trackers and can have parent-child relationships with other work items:

* **Work Items field** in META items lists specific work items that fall under that phase
* The script tracks these relationships and labels non-META items with their parent META IDs in WORK_ITEMS.md
* META items can be blocked by other META items listed in their Work Items field
* This allows tracking of project phases and milestones

## Generated Documentation

The script generates two documentation files in the `docs/` directory:

### WORK_ITEMS.md

Master index of all work items (excluding IDEA prefix items), organized by priority and status. Features:

* Reference-style links to work item files
* Items grouped by status (Open, In Progress, Blocked) and priority
* META items shown first as phase trackers
* Non-META items labeled with their parent META IDs when applicable
* Headerless link glossary at the end for easy reference
* "Updated on" timestamp showing when the file was last regenerated
* Only rewritten if content has changed (ignoring timestamp)

### FUTURE_IDEAS.md

Separate index for IDEA prefix items, which are potential future work rather than active tasks. Features:

* Similar format to WORK_ITEMS.md but focused on ideas
* Useful for brainstorming and future planning
* Also includes "Updated on" timestamp
* Only rewritten if content has changed (ignoring timestamp)
