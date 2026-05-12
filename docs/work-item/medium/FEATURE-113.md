# FEATURE-113: In-process Typst PDF compilation (replace typst CLI subprocess)

## Summary

Replace the `std::process::Command::new("typst")` subprocess calls in
`schedule-layout` and `cosam-convert` with in-process compilation using the
`typst` Rust crate, eliminating the external `typst-cli` dependency.

## Status

Open

## Priority

Medium

## Description

Both `apps/cosam-convert/src/main.rs` (`run_layout_export`) and
`apps/cosam-layout/src/main.rs` (`compile_typst`) currently shell out to the
`typst compile` CLI binary to produce PDFs. This requires `typst-cli` to be
installed separately and on `PATH`, which is inconvenient and fragile.

The `typst` Rust crate provides a `compile()` API that can do this in-process,
but it requires implementing the `World` trait (file I/O, font loading, date,
package resolution). The `typst-kit` crate (maintained by the Typst team)
provides ready-made font search and embed helpers to simplify `World`
implementation.

## Implementation Details

- Add a `compile` feature to `schedule-layout` (already stubbed in `Cargo.toml`)
  that enables `typst`, `typst-library`, `typst-pdf`, and `comemo` dependencies
- Implement a `ScheduleWorld` struct satisfying `typst::World` in a new
  `crates/schedule-layout/src/compile.rs` module:
  - `main()` — returns the `.typ` source as the main file
  - `source()` / `file()` — no external file access needed for embedded sources
  - `font()` — use `typst-kit` font searcher seeded with `brand.fonts.font_dir`
  - `today()` — return current UTC date
  - `library()` / `book()` — delegate to `typst-library` defaults
- Replace the `Command::new("typst")` block in both binaries with a call to the
  new in-process function; keep subprocess as fallback when feature is disabled
- Enable the `compile` feature by default in `schedule-layout/Cargo.toml`
- Note: `typst-kit` version must match the `typst` version in use (0.13.x)

## Acceptance Criteria

- `cargo build -p schedule-layout --features compile` succeeds
- `cosam-layout --input schedule.json --format schedule --paper tabloid` produces
  a PDF without `typst-cli` installed
- `cosam-convert --export-layout <dir>` produces PDFs without `typst-cli`
- `cargo build -p schedule-layout` (no `compile` feature) still compiles cleanly
  and falls back to the subprocess path
