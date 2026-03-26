# ScheduleFile XLSX Subclassing

## Summary

Refactor xlsx module to be a specialization/implementation detail of ScheduleFile

## Status

Open

## Priority

Medium

## Description

Currently `xlsx/mod.rs` exposes `load_auto` and `save_auto` as public functions. A cleaner architecture would make these methods on `ScheduleFile` itself, with xlsx as an internal implementation detail.

### Proposed API

```rust
impl ScheduleFile {
    pub fn load_auto(path: &Path, options: &XlsxImportOptions) -> Result<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("xlsx") => {
                // Internal: call xlsx::import_xlsx
            }
            Some("json") => {
                // Internal: call Self::load
            }
            _ => Self::load(path) // Default to JSON
        }
    }
    
    pub fn save_auto(&mut self, path: &Path) -> Result<()> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("xlsx") => {
                // Internal: call xlsx::update_xlsx
            }
            _ => {
                // Internal: call self.save_json
            }
        }
    }
}
```

### Implementation Steps

1. Move `load_auto` and `save_auto` from `xlsx/mod.rs` to `file/mod.rs` as `ScheduleFile` methods
2. Make xlsx functions `pub(crate)` (internal to crate)
3. Update all applications to use `ScheduleFile::load_auto` and `ScheduleFile::save_auto`
4. Remove `xlsx/mod.rs` dispatch functions
5. Update documentation

### Benefits

- Single entry point for file I/O through `ScheduleFile`
- Cleaner public API
- Easier to add new file formats in future
- Better encapsulation of format-specific logic

## Acceptance Criteria

- All xlsx functions become internal to the crate
- `ScheduleFile` provides the only public file I/O API
- All applications updated to use new API
- Tests pass
- Documentation updated
