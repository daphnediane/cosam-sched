# Support Hide Panelist and Alt Panelist fields

## Summary

The converter ignores the "Hide Panelist" and "Alt Panelist" spreadsheet columns, so presenter suppression and override text are not honored in the JSON output.

## Status

Open

## Priority

High

## Description

In the schedule-to-html spreadsheet format, two columns control presenter display:

- **Hide Panelist**: When non-blank (e.g. "Yes" or "*"), the event's presenter
  list should be suppressed entirely. This is used for events where listing the
  panelists is not appropriate (e.g. staff-run logistics panels).

- **Alt Panelist**: When set, the computed presenter list is replaced with this
  text (e.g. "Mystery Guest"). Useful for one-off presenters who don't have
  their own column or for special display.

Currently `Events.pm` reads presenter columns but never checks these fields,
so all detected presenters are unconditionally included in the JSON output.

See also: `docs/spreadsheet-format.md` and schedule-to-html README §Panelist.

## Implementation Details

### Converter (`Events.pm`)

1. After building `@event_presenters`, check `$data->{Hide_Panelist}`:
   - If non-blank, clear `@event_presenters` (set to empty array)
   - Do NOT add these presenters to the global `%presenter_set`

2. Check `$data->{Alt_Panelist}`:
   - If set, replace `@event_presenters` with a single-element array
     containing the alt text
   - The alt text should NOT be added to the global presenter set

3. Precedence: `Hide_Panelist` suppresses everything; `Alt_Panelist` only
   applies if `Hide_Panelist` is not set.

### Widget (`cosam-calendar.js`)

- No widget changes needed — the widget already handles empty presenter
  arrays and displays whatever the JSON contains.

### JSON output

- `presenters` field remains an array of strings; when hidden it is `[]`,
  when alt is used it is `["Mystery Guest"]` (or whatever the alt text is).

## Testing

- Verify that events with "Hide Panelist" = "Yes" have empty presenter arrays
- Verify that events with "Alt Panelist" = "Mystery Guest" show that text
- Verify that hidden/alt presenters do not appear in the global presenters list
- Verify normal events are unaffected
