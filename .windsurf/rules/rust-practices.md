---
description: Rust coding practices for the editor crate
---

# Rust Coding Practices

## General

- Target the latest stable Rust toolchain.
- Run `cargo clippy` before committing; treat warnings as errors.
- Format all code with `rustfmt` (default settings).
- Prefer `thiserror` for library error types and `anyhow` for application-level error handling.

## Code Style

- Use `snake_case` for functions, methods, variables, and modules.
- Use `CamelCase` for types and traits.
- Use `SCREAMING_SNAKE_CASE` for constants and statics.
- Prefer explicit types on public API boundaries; elide types in local bindings when obvious.
- Keep functions short and focused; extract helpers rather than nesting deeply.
- Avoid `unwrap()` and `expect()` in non-test code; propagate errors with `?`.
- Use `#[must_use]` on functions whose return value should not be silently ignored.

## Derive and Traits

- Derive `Debug` on all public types.
- Derive `Clone`, `PartialEq` where meaningful for data types.
- Derive `Serialize`, `Deserialize` for types that cross serialization boundaries.
- Use `#[serde(rename_all = "camelCase")]` to match the existing JSON schema.

## Module Organization

- One primary type per file; related small types may share a file.
- Re-export public items from `mod.rs` for clean external API.
- Keep `use` statements grouped: std, external crates, crate-internal.

## Testing

- Every data module must have a `#[cfg(test)] mod tests` block.
- Test serialization round-trips for all data structs.
- Use `include_str!` or test fixtures in a `tests/` directory for JSON samples.
- Name tests descriptively: `test_event_deserialize_minimal`, not `test1`.
- Run tests with `cargo test` before committing.

## Dependencies

- Pin major versions in `Cargo.toml` (e.g., `serde = "1"`).
- Keep the dependency tree minimal; prefer well-maintained crates.
- Use feature flags to gate optional functionality (e.g., XLSX support).

## GPUI Specifics

- Implement `Render` trait for view structs.
- Use GPUI's entity system (`cx.new(...)`) for state management.
- Prefer declarative `div()` builder API over raw element implementations.
- Keep view logic in `ui/` modules, data logic in `data/` modules.
