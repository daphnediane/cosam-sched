# Preferred UUIDs with Context-Dependent Collision

## Summary

Implement V5 UUID generation from natural keys with entity-type-specific collision handling for import scenarios.

## Status

Not Started

## Priority

High

## Description

Enable providing preferred UUIDs when adding entities, with collision semantics that vary by entity type. Panels get new V7 UUIDs on collision (duplicate spreadsheet entries), while reference data updates existing entities.

## Implementation Details

### 1. Define UuidPreference and OnCollision

**File: `entity/mod.rs` or new `entity/uuid.rs`**

```rust
pub enum UuidStrategy {
    GenerateV7,                    // Standard new UUID
    FromV5 { namespace: Uuid, name: String },  // Deterministic from natural key
    Exact(NonNilUuid),             // Use this exact UUID
}

pub enum OnCollision {
    Error,           // Return error if UUID exists
    GenerateNew,     // Generate new V7 UUID (for Panels)
    UpdateExisting,  // Update fields on existing entity (for reference data)
}

pub struct UuidPreference {
    pub strategy: UuidStrategy,
    pub on_collision: OnCollision,
}
```

### 2. Extend EntityBuilder

**File: `schedule-macro/src/lib.rs`** (from REFACTOR-053)

- Add `with_uuid_preference()` method to builders
- Default: `GenerateV7` with `Error` on collision
- Store preference in builder struct

### 3. Implement Collision Handling in Storage

**File: `schedule/storage.rs`**

- Check for UUID existence before insert
- Apply entity-type-specific collision behavior:
  - Panel: GenerateNew → create new with V7, return both IDs to caller
  - Presenter: Error (caller should use match_index first)
  - EventRoom/HotelRoom: UpdateExisting → merge fields
  - PanelType: UpdateExisting → merge fields

### 4. Import Helper Methods

**File: `schedule/mod.rs`**

```rust
pub fn import_panel_with_presenters(&mut self, panel_data: PanelBuilder, presenter_names: Vec<String>) -> Result<(PanelId, Vec<PresenterId>), _> {
    // Handle panel UUID preference
    // For each presenter: match_index first, then add_or_update
    // Bundle all operations
}
```

## Acceptance Criteria

- [ ] UuidPreference and OnCollision enums defined
- [ ] EntityBuilder supports with_uuid_preference()
- [ ] V5 UUID generation from natural keys works
- [ ] Panel collision → new V7 UUID with both IDs returned
- [ ] Reference data collision → update existing entity
- [ ] Presenter import pattern: match_index → add_or_update documented
- [ ] Import scenarios tested (GW023 duplicate, Jane Doe merge)

## Dependencies

- REFACTOR-053 (Builder & Partial Updates) - needs builder foundation

## Notes

Context-dependent collision handling is crucial for spreadsheet imports:

- GW023 appearing twice = duplicate entry → new UUID for second
- Jane Doe appearing twice = same person → merge/update

When complete, update REFACTOR-051.md to mark REFACTOR-054 as complete.
