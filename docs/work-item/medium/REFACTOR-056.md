# CLI Field Access

## Summary

Enable dynamic field access by string name for CLI tools, supporting field updates without compile-time entity knowledge.

## Status

Not Started

## Priority

Medium

## Description

Implement dynamic field lookup and modification for command-line utilities like cosam-modify. CLI tools can specify entities and fields by name, with values as strings that are parsed to appropriate types.

## Implementation Details

### 1. DynamicField Trait

**File: `field/dynamic.rs`**

```rust
pub trait DynamicField: NamedField {
    fn parse_from_str(&self, value: &str) -> Result<FieldValue, ParseError>;
    fn type_name(&self) -> &'static str;
    fn is_required(&self) -> bool;
}
```

### 2. Extend FieldSet

**File: `entity/mod.rs` (FieldSet impl)**

```rust
impl FieldSet {
    pub fn get_dynamic_field(&self, name: &str) -> Option<&dyn DynamicField>;
    pub fn field_names(&self) -> Vec<&'static str>;
    pub fn required_fields(&self) -> Vec<&'static str>;
}
```

### 3. FieldValue String Parsing

**File: `field/mod.rs`**

```rust
impl FieldValue {
    pub fn from_str_with_type(value: &str, type_hint: FieldTypeCategory) -> Result<Self, ParseError>;
}
```

### 4. Schedule Dynamic Field Methods

**File: `schedule/mod.rs`**

```rust
impl Schedule {
    pub fn set_field_by_name(
        &mut self,
        entity_id: EntityUUID,
        field_name: &str,
        value_str: &str,
    ) -> Result<(), FieldError>;
    
    pub fn get_field_by_name(
        &self,
        entity_id: EntityUUID,
        field_name: &str,
    ) -> Result<FieldValue, FieldError>;
}
```

### 5. Macro Generation

**File: `schedule-macro/src/lib.rs`**

- Generate DynamicField implementations for each field
- Register fields in FieldSet with type metadata

### 6. CLI Tool Framework

**New crate: `cosam-modify`** (or update existing)

```rust
// Example CLI usage:
// cosam-modify --schedule schedule.json set panel:GP001 description "New description"
// cosam-modify --schedule schedule.json add presenter --name "Jane Doe" --bio "Guest"
```

## Acceptance Criteria

- [ ] DynamicField trait implemented for all field types
- [ ] FieldSet supports get_dynamic_field() lookup
- [ ] FieldValue string parsing with type conversion
- [ ] Schedule::set_field_by_name works for all entity types
- [ ] Schedule::get_field_by_name works for all entity types
- [ ] Edge-entity metadata accessible via dynamic fields
- [ ] CLI tool can add/update entities by field name
- [ ] All existing tests pass

## Dependencies

- REFACTOR-053 (Builder & Partial Updates) - dynamic fields wrap builder operations
- REFACTOR-055 (Transaction System) - CLI operations should be transactional

## Notes

This enables the "new version of cosam-modify" requirement - CLI tools that don't need internal entity knowledge.

When complete, update REFACTOR-051.md to mark REFACTOR-056 as complete.
