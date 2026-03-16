# JSON Export and Save

## Summary

Implement saving the schedule as JSON, matching the format consumed by the widget.

## Status

Open

## Priority

High

## Description

Allow saving the in-memory schedule back to `schedule.json` format. This enables the editor to serve as the primary authoring tool, with output directly usable by the web widget.

## Implementation Details

- Serialize Schedule to JSON matching existing `sample-data.json` schema
- Support Save (overwrite current file) and Save As (new path)
- Keyboard shortcuts: Cmd+S / Ctrl+S for Save
- Pretty-print JSON for readability
- Update `meta.generated` timestamp on save
- Warn before overwriting if file has been modified externally
