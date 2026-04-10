# Cargo Workspace Setup With Crate Skeletons

## Summary

Set up the Cargo workspace root and create skeleton crates for all planned components.

## Status

Open

## Priority

High

## Description

Initialize `cosam_sched` as a Cargo workspace with the following layout:

```text
Cargo.toml              (workspace root)
crates/
  schedule-data/        (core data model, entities, fields, storage)
  schedule-macro/       (proc-macro crate for #[derive(EntityFields)])
apps/
  cosam-convert/        (format conversion CLI)
  cosam-modify/         (CLI editing tool)
  cosam-editor/         (GUI editor — skeleton only)
```

Each crate should have:

- `Cargo.toml` with `license = "BSD-2-Clause"` and `authors = ["Daphne Pfister"]`
- Copyright header in all source files
- Minimal `lib.rs` or `main.rs` that compiles

## Acceptance Criteria

- `cargo build` succeeds at workspace root
- `cargo test` succeeds (no tests yet, but no compile errors)
- All Cargo.toml files have correct license and author metadata
