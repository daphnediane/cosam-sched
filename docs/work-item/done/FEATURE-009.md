# Cargo Workspace Setup With Application Skeletons

## Summary

Set up the Cargo workspace root and create skeleton application crates.

## Status

Completed

## Priority

High

## Description

Initialize the Cargo workspace with the following layout:

```text
Cargo.toml              (workspace root)
crates/
  schedule-core/        (empty — populated in Phase 2)
apps/
  cosam-convert/        (format conversion CLI skeleton)
  cosam-modify/         (CLI editing tool skeleton)
  cosam-editor/         (GUI editor skeleton)
```

Each crate has:

- `Cargo.toml` with `license = "BSD-2-Clause"` and `authors = ["Daphne Pfister"]`
- Copyright header in all source files
- Minimal `lib.rs` or `main.rs` that compiles

## Acceptance Criteria

- `cargo build` succeeds at workspace root
- `cargo test` succeeds (no tests yet, but no compile errors)
- All Cargo.toml files have correct license and author metadata
