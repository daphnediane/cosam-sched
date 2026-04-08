# Builder & Partial Updates

## Summary

Extend EntityFields macro to generate `EntityBuilder<T>` for partial entity construction and updates without requiring all fields.

## Status

Not Started

## Priority

High

## Description

Implement builder pattern for all entity types (including edge-entities from REFACTOR-052). This enables partial updates, required field validation, and cleaner entity construction APIs. Replaces the mutation family approach from REFACTOR-004.

## Implementation Details

### 1. Extend EntityFields Macro

**File: `schedule-macro/src/lib.rs`**

- Generate `EntityBuilder<T>` struct for each entity
- Generate setter methods for each field: `with_field_name(value)`
- Generate `build()` method with required field validation
- Generate `into_data()` to create entity data struct
- Generate `apply_to()` for updating existing entities

### 2. Builder Structure

```rust
pub struct PanelBuilder {
    uuid_preference: Option<UuidPreference>,
    uid: Option<String>,
    name: Option<String>,
    description: Option<String>,
    // ... all fields as Option<T>
}

impl PanelBuilder {
    pub fn new() -> Self { ... }
    pub fn with_uid(mut self, uid: impl Into<String>) -> Self { ... }
    pub fn with_name(mut self, name: impl Into<String>) -> Self { ... }
    // ... generated setters
    
    pub fn build(self) -> Result<PanelData, BuilderError> { 
        // validate required fields present
    }
    
    pub fn into_data(self) -> Result<PanelData, BuilderError> { ... }
    pub fn apply_to(self, data: &mut PanelData) -> Result<(), BuilderError> { ... }
}
```

### 3. Update Schedule Methods

**File: `schedule/mod.rs`**

```rust
add_entity<T: EntityType>(&mut self, builder: impl EntityBuilder<T>) -> Result<NonNilUuid, _>
update_entity<T: EntityType>(&mut self, id: TypedId<T>, f: impl FnOnce(&mut T::Builder)) -> Result<(), _>
```

### 4. Edge-Entity Builder Support

- Ensure builders work for edge-entities from REFACTOR-052
- Support creating panel + edges in single bundle

## Acceptance Criteria

- [ ] EntityFields macro generates `EntityBuilder<T>` for all entities
- [ ] Builder supports all field setters with type-safe conversions
- [ ] Required field validation at build() time
- [ ] Schedule::add_entity accepts builders
- [ ] Schedule::update_entity accepts builder closure for partial updates
- [ ] Edge-entity builders work correctly
- [ ] All existing tests pass with new APIs
- [ ] REFACTOR-004 marked as superseded/complete

## Dependencies

- REFACTOR-052 (Edge to Edge-Entity) - edge-entity builders need this first

## Notes

This phase supersedes REFACTOR-004 (Mutation API). The builder pattern provides the same "mutation families" capability (add, update, restore, find-or-add) but through a more idiomatic Rust pattern.

When complete, update REFACTOR-051.md to mark REFACTOR-053 as complete.
