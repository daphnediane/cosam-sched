# Extended Config File Handling

## Summary

Extend the `DeviceConfig` / `identity.toml` system with richer identity fields,
per-app metadata, and optional named profiles.

## Status

Open

## Priority

Low

## Description

The basic config system stores a display name and per-app actor UUIDs. This idea
records extensions deferred from the initial implementation:

### Richer identity fields

- `real_name`, `public_email`, `role` (Programming, Director, etc.)
- `base_uuid` (UUID v7, generated once, stable human identity across devices)

### Per-app config metadata

- App-specific key/value pairs in `[meta]` section of `actor-<app>.toml`
- e.g., `last_open_dir`, `app_version`

### Actor UUID derivation

- UUID v5 derived from `base_uuid` + app name (deterministic, no file needed)
- Per-session UUIDs (UUID v7) for batch CLI tools

### Named profiles

- Subdirectories for role-specific identity overrides
- Selected via CLI flag or env var

### When to implement

When the CRDT layer (Phase 3) needs richer actor metadata for change attribution.
