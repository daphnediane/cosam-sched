# cosam-convert CLI Reference

`cosam-convert` imports schedule data from XLSX spreadsheets and exports it to
multiple output formats in a single invocation. Output settings accumulate and
are snapshotted per output command, so one run can produce outputs with different
titles, minification, or widget resources.

## Synopsis

```text
cosam-convert --input <file> [settings...] <output-command> [[settings...] <output-command>]...
cosam-convert --input <file> --check
```

## Input

| Flag                          | Description                                                        |
| ----------------------------- | ------------------------------------------------------------------ |
| `--input <file>`, `-i <file>` | Input file — `.xlsx` XLSX spreadsheet or native `.schedule` binary |

If `<file>` does not start with `-` and no `--input` flag has been seen, it is
treated as the input path (positional shorthand).

## Output Commands

Each output command captures a snapshot of the current settings at the point it
appears. Multiple commands of any type may be mixed in one invocation.

| Flag                                     | Description                                                                                                                                                                                            |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `--output <file>`, `-o <file>`           | Save private schedule. Extension determines format: `.xlsx` → XLSX round-trip; anything else → native CRDT binary.                                                                                     |
| `--export <file.json>`, `-e <file.json>` | Export public widget JSON (see [widget-json-format.md](widget-json-format.md)).                                                                                                                        |
| `--export-embed <file.html>`             | Self-contained embeddable HTML snippet — inline CSS, JS, and schedule data; no external dependencies. Paste into a Squarespace Code Block. Format controlled by `--embed-as-html` / `--embed-as-json`. |
| `--export-test <file.html>`              | Standalone test page simulating a Squarespace Bedford-family site with the widget embedded.                                                                                                            |
| `--export-xlsx-grid <file.xlsx>`         | Export only the per-day grid reference sheets (one sheet per logical day), omitting the data tables that `--output <file>.xlsx` includes.                                                              |

## Validation

| Flag                    | Description                                                                                                                                                |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--check`, `--validate` | Load and report scheduling conflicts, then exit. Exits non-zero if conflicts are found. May be combined with output commands to validate before exporting. |

## XLSX Table Names

Override the sheet or table names used when importing from XLSX. The importer
tries the given name first, then falls back to common aliases.

| Flag                       | Default      | Fallback aliases |
| -------------------------- | ------------ | ---------------- |
| `--schedule-table <name>`  | `Schedule`   | —                |
| `--roommap-table <name>`   | `RoomMap`    | `Rooms`          |
| `--prefix-table <name>`    | `Prefix`     | `PanelTypes`     |
| `--presenter-table <name>` | `Presenters` | `People`         |

## Timezone and Schedule Window

The schedule's timezone and event-window bounds are read from the source's
**Meta** / **Timestamp** sheet when present (see
[spreadsheet-format.md](spreadsheet-format.md#meta--timestamp-sheet)). These
flags supply *defaults* used only for fields the source leaves unset.

| Flag                            | Description                                                                                  |
| ------------------------------- | -------------------------------------------------------------------------------------------- |
| `--default-timezone <name>`     | IANA name (`America/New_York`) or abbreviation (`EDT`, `UTC`). Defaults to the system local zone. |
| `--default-start-time <dt>`     | Schedule-window start; extended earlier by any panel scheduled before it.                    |
| `--default-end-time <dt>`       | Schedule-window end; extended later by any panel scheduled after it.                         |

All naive timestamps in the schedule are interpreted as wall-clock in the
resolved timezone, which is embedded in exported widget JSON/HTML metadata and
used to anchor `.ics` (Add to Calendar) downloads. To set these values
authoritatively on a `.schedule` file (rather than as defaults), use
`cosam-modify --set-timezone` / `--set-start-time` / `--set-end-time`.

## Output Settings

Settings apply to all subsequent output commands until overridden or reset.
They do **not** affect output commands that appear before them.

### Title

| Flag               | Description                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------------- |
| `--title <string>` | Event title embedded in widget JSON metadata and used as the page title in `--export-test` pages. |

### Widget Resources

The built-in CSS, JS, and test-page template are compiled into the binary. Use
these flags to override with files on disk.

| Flag                     | Description                                                                                                                          |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `--widget <basename>`    | Set both CSS and JS from `<basename>.css` and `<basename>.js`.                                                                       |
| `--widget-css <path>`    | Override CSS source. Accepts a file path, a path without extension, or a directory (looks for `cosam-calendar.css` inside).          |
| `--widget-js <path>`     | Override JS source. Same path resolution as `--widget-css`.                                                                          |
| `--test-template <path>` | Override the Squarespace simulation HTML template used by `--export-test`. Must contain `{WIDGET_BLOCK}` and `{TITLE}` placeholders. |

#### Reset to builtins

| Flag                 | Resets                                                               |
| -------------------- | -------------------------------------------------------------------- |
| `--builtin-css`      | CSS                                                                  |
| `--builtin-js`       | JS                                                                   |
| `--builtin-widget`   | CSS and JS                                                           |
| `--builtin-template` | Test template                                                        |
| `--builtin`          | CSS, JS, and template                                                |
| `--default`          | All settings (CSS, JS, template, title, minification, stylePageBody) |

### Embed Format

Controls how schedule data is embedded in `--export-embed` and `--export-test` output.

| Flag              | Description                                                                                                           |
| ----------------- | --------------------------------------------------------------------------------------------------------------------- |
| `--embed-as-html` | Embed as widget-html: compact JSON block for structural data plus semantic `<article>` elements for panels (default). |
| `--embed-as-json` | Embed as gzip+base64 JSON (legacy format, compatible with older widget builds).                                       |

### Minification

| Flag                           | Description                                         |
| ------------------------------ | --------------------------------------------------- |
| `--minified`                   | Minify HTML output — CSS, JS, and markup (default). |
| `--no-minified`, `--for-debug` | Skip minification for human-readable output.        |

### Widget Initialization

| Flag              | Description                                                                                        |
| ----------------- | -------------------------------------------------------------------------------------------------- |
| `--style-page`    | Pass `stylePageBody: true` to the widget initializer (applies Squarespace-compatible body styles). |
| `--no-style-page` | Pass `stylePageBody: false`.                                                                       |

If neither `--style-page` nor `--no-style-page` is set, the `stylePageBody`
parameter is omitted from the initializer call.

## Validation Rules

- `--input` is required.
- At least one output command is required unless `--check` is specified.
- The same output path may not be specified more than once.
- Settings flags (`--title`, `--widget-*`, `--minified`, etc.) that appear after
  the last output command are an error (orphaned settings).

## Conflict Detection

`--check` (or `--validate`) detects two kinds of scheduling conflicts:

- **Room conflicts** — two panels assigned to the same room with overlapping
  time ranges.
- **Presenter conflicts** — the same presenter credited on two panels with
  overlapping time ranges.

Conflict output goes to stderr. The exit code is 0 if no conflicts are found,
1 if conflicts are found.

## Examples

### Export public JSON and embeddable HTML

```bash
cosam-convert --input "My Event.xlsx" \
  --title "My Event 2026 Schedule" \
  --export public.json \
  --export-embed embed.html \
  --export-test test.html
```

### Validate only

```bash
cosam-convert --input "My Event.xlsx" --check
```

### Validate, then export if clean

The `--check` flag does not prevent subsequent output commands — it only changes
the exit code. To stop on conflict, validate in a separate step:

```bash
cosam-convert --input "My Event.xlsx" --check && \
cosam-convert --input "My Event.xlsx" --export public.json
```

### Multiple outputs with different settings in one pass

```bash
cosam-convert --input schedule.xlsx \
  --title "Event 2026" \
  --output schedule.xlsx \
  --export public.json \
  --minified   --export-embed embed.html \
               --export-test  test.html \
  --style-page --export-embed style-embed.html \
               --export-test  style-page.html
```

This is how `scripts/export-schedules.sh` builds all six outputs per year.

### Override widget resources

```bash
cosam-convert --input schedule.xlsx \
  --widget-css ./custom/my-theme.css \
  --widget-js  ./custom/my-widget.js \
  --export-embed custom-embed.html
```

## Output File Summary

The batch export scripts (`scripts/export-schedules.sh` /
`scripts/export-schedules.ps1`) produce these files per schedule year:

| File               | Flag                                    | Notes                                       |
| ------------------ | --------------------------------------- | ------------------------------------------- |
| `schedule.xlsx`    | `--output`                              | XLSX round-trip copy of input               |
| `public.json`      | `--export`                              | Public widget JSON                          |
| `embed.html`       | `--export-embed`                        | Embeddable HTML, minified, no stylePageBody |
| `test.html`        | `--export-test`                         | Test page, minified, no stylePageBody       |
| `style-embed.html` | `--export-embed` (after `--style-page`) | Embeddable HTML with stylePageBody          |
| `style-page.html`  | `--export-test` (after `--style-page`)  | Test page with stylePageBody                |

## See Also

- [spreadsheet-format.md](spreadsheet-format.md) — XLSX column reference
- [widget-json-format.md](widget-json-format.md) — exported JSON schema
- [widget/README.md](../widget/README.md) — calendar widget embedding guide
