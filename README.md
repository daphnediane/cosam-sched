# cosam-sched

Embeddable interactive event schedule system for conventions and multi-track events.
Imports schedule data from XLSX spreadsheets, exports public-facing JSON for the
calendar widget, and produces self-contained HTML pages ready to embed or test.

Originally developed for [Cosplay America](https://cosplayamerica.com/), and
open to use by any event that fits the spreadsheet format.

## Quick Start

### Convert a Schedule

```bash
cosam-convert --input "My Event.xlsx" \
  --check \
  --title "My Event 2026 Schedule" \
  --export public.json \
  --export-embed embed.html \
  --export-test test.html
```

- `--check` validates for room and presenter conflicts before exporting
- `--export` writes the public widget JSON
- `--export-embed` writes a self-contained HTML snippet for embedding
- `--export-test` writes a standalone page for local testing

See [docs/cosam-convert.md](docs/cosam-convert.md) for the full CLI reference.

## Using with Your Own Event

The schedule format is a multi-sheet XLSX spreadsheet — the same format as the
original [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project.
If you want to run this for your own convention or multi-track event, start with
[docs/spreadsheet-format.md](docs/spreadsheet-format.md).

A desktop editor for creating and collaborating on schedules is planned (see
`cosam-editor` in the component list below).

## Batch Export

The included scripts rebuild all years at once, reading from
`input/<YEAR> Schedule.xlsx` and writing six output files per year to
`output/<YEAR>/`. Edit the script to set the event title for your organization:

```bash
scripts/export-schedules.sh        # bash / macOS / Linux
scripts/export-schedules.ps1       # PowerShell / Windows
```

## Components

| Component       | Description                                       |
| --------------- | ------------------------------------------------- |
| `cosam-convert` | CLI for importing XLSX and exporting JSON / HTML  |
| `cosam-modify`  | CLI for editing schedule data                     |
| `cosam-editor`  | Desktop GUI editor (in development)               |
| `widget/`       | Embeddable JavaScript calendar widget             |

## Spreadsheet Format

The schedule is defined across up to four sheets in an XLSX workbook:

- **Schedule** (or any name set via `--schedule-table`): panels and their timing,
  room, type, and presenter assignments
- **RoomMap** / **Rooms**: room names, sort keys, and hotel room mappings
- **Prefix** / **PanelTypes**: panel type definitions with colors and flags
- **Presenters** / **People**: presenter classifications and group memberships

See [docs/spreadsheet-format.md](docs/spreadsheet-format.md) for the full column
reference.

## Widget

The calendar widget is a self-contained JavaScript/CSS package that renders the
exported JSON as an interactive filterable schedule grid. See
[widget/README.md](widget/README.md) for embedding instructions.

## Documentation

User and developer documentation lives in [docs/doc-index.md](docs/doc-index.md).

## License

Copyright (c) 2026 Daphne Pfister. Licensed under the
[BSD-2-Clause License](LICENSE).

## Attribution

This project is a rewrite of and based on the original
[schedule-to-html](https://github.com/daphnediane/schedule-to-html) project.

## AI Coding Declaration

Development of this project has been assisted by AI coding tools:

- [Windsurf](https://windsurf.com/)
- [Claude Code](https://claude.ai/code)

### A note of my goals, or why AI

I'm primarily a software tools developer normally focused on compilers, and
am using this project to expand my familiarity with AI coding tools as well
as learn the rust programming language. I've tried to audit all the generate
code manually, especially since I want to figure out anti-patterns and the
best way to figure out the right way to use a new language is to learn what
one shouldn't do. I've actually done multiple versions of this software,
resetting back to ground zero a few times just to try out different
approaches. Just a heads up that until I get everything to a 1.0 place
this code might see massive reorganizations and rewrites. As I wrote this
note I just had Claude re-implement XLSX exports for the 5th or 6th time.
Still too many magic numbers for my taste, the code is both over and under
engineered at the same time.

I'm still mixed opinion on AI but its here. It's a strange time to be a
programmer.

Oh and I should note that a bunch of the documentation is also written by
Windsurf and Claude, I've been trying to keep it to be concise, but it might
get a bit overly verbose at times. Take the documentation as a slightly out
of date roadmap, no matter the polish of the AI verbiage. Here be dragons.

I'd normally toss a few em-dashes in something like this, but that would
make this part sound like it was written by AI—instead of by me.

AI did in fact write most of this README, but this note section all me.
