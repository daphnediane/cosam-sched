# FEATURE-113: In-process Typst PDF compilation (replace typst CLI subprocess)

## Summary

Replace the `std::process::Command::new("typst")` subprocess call in
`cosam-convert` with in-process compilation using the `typst` Rust crate,
eliminating the external `typst-cli` dependency.

## Status

Open

## Priority

High

## Description

`apps/cosam-convert/src/main.rs` (`run_layout_export`) currently shells out to
the `typst compile` CLI binary to produce PDFs. This requires `typst-cli` to be
installed separately and on `PATH`, which is inconvenient and fragile.
(`cosam-layout` was removed in CLI-139; layout rendering now lives entirely in
`cosam-convert`.)

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
- Replace the `Command::new("typst")` block in `cosam-convert` with a call to
  the new in-process function; keep subprocess as fallback when feature is
  disabled
- Enable the `compile` feature by default in `schedule-layout/Cargo.toml`
- Note: `typst-kit` version must match the `typst` version in use (0.13.x)

## Acceptance Criteria

- `cargo build -p schedule-layout --features compile` succeeds
- `cosam-convert --export-layout <dir>` produces PDFs without `typst-cli`
  installed
- `cargo build -p schedule-layout` (no `compile` feature) still compiles cleanly
  and falls back to the subprocess path
