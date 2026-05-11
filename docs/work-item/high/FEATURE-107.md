# FEATURE-107: Add apps/cosam-layout binary

## Summary

New CLI binary that consumes `schedule.json` and `config/brand.toml` to produce Typst-compiled PDFs and/or `.typ` source files for all print layout formats.

## Status

Open

## Priority

High

## Blocked By

- FEATURE-106: Requires schedule-layout crate

## Description

Create `apps/cosam-layout` as a workspace member. CLI is modeled after `cosam-convert` style with repeatable layout job args separated by `--`.

## Implementation Details

### CLI interface

```text
cosam-layout [OPTIONS] --input <schedule.json> [-- LAYOUT_ARGS...]

Global options:
  --input <FILE>          Input schedule.json
  --output-dir <DIR>      Output directory [default: output/layout]
  --brand-config <FILE>   Brand config [default: config/brand.toml]
  --typ                   Also write .typ source files alongside PDFs
  --no-compile            Write .typ only, skip PDF compilation
  --dump-sample-brand     Print sample brand.toml to stdout and exit
  --color-mode <MODE>     color|bw [default: color]

Per-layout args (repeatable with --):
  --format <FORMAT>       schedule|workshop-poster|room-signs|guest-postcards|descriptions
  --paper <SIZE>          legal|tabloid|super-b|postcard-4x6
  --split <MODE>          day|half-day
  --filter-premium        Workshop-poster: premium only
  --filter-room <ID>      Room-signs: specific room UID
  --filter-guest <NAME>   Guest-postcards: specific guest name
  --output <FILE>         Override output file name
```

### Output naming

`{format}-{paper}-{split}-{qualifier}.pdf`

Examples:

- `schedule-tabloid-half-day-friday-morning.pdf`
- `room-signs-tabloid-salon-a.pdf`
- `guest-postcards-4x6-saturday-morning-john-doe.pdf`

### Formats covered

- `schedule` — grid left + descriptions right; Legal or Tabloid (tri-fold); double-sided; splits by day or half-day
- `workshop-poster` — premium workshops; QR codes from `ticketUrl`; Tabloid
- `room-signs` — per room per day; grid with room highlighted; Tabloid or Super B
- `guest-postcards` — 4×6 per guest per half-day; multiple cards per sheet
- `descriptions` — description-only multi-column; one side per day; Tabloid or Super B

## Acceptance Criteria

- `cosam-layout --input schedule.json --format schedule --paper tabloid` produces a PDF
- `--no-compile` writes `.typ` only; `--typ` writes both
- `--dump-sample-brand` prints TOML to stdout
- Missing `brand.toml` warns and falls back to built-in defaults
- All 5 formats produce non-empty output without panicking
