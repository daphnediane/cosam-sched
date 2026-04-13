# FEATURE-045: Work item script rename and --create command

## Summary

Rename combine-workitems.pl to work-item-update.pl and add a --create flag
to generate properly numbered placeholder work item files from templates.

## Status

Completed

## Priority

Medium

## Description

The combine-workitems.pl script was renamed to work-item-update.pl to better
reflect its expanded role. A --create flag was added to create properly
numbered placeholder work item files without needing to manually look up the
next available ID.

## Implementation Details

- Renamed scripts/combine-workitems.pl to scripts/work-item-update.pl
- Added --create PREFIX flag (repeatable; comma-separated also accepted)
- Added %known_prefixes validation with helpful error listing supported tags
- Added Placeholder status routed to docs/work-item/new/ subdirectory
- Added docs/work-item/template/ directory with default, META, and IDEA templates
- Both find() calls skip the template/ directory
- Template loading: PREFIX-template.md → default-template.md → hardcoded fallback
- Updated references in execution-rhythm.md, track_work_item.md, prepare-comment.md
- Updated track_work_item.md workflow to document --create usage and new/ subdir
