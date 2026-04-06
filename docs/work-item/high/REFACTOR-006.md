# Scheduling Derivation and Field Validation

## Summary

Implement derived scheduling-state propagation and complete the field validation system in schedule-macro.

## Status

Not Started

## Priority

High

## Description

Implement derived scheduling state (scheduled/unscheduled) based on time_range presence and indirect references (presenter groups). Complete the `#[validate]` attribute in schedule-macro to generate `CheckedField` implementations.

## Implementation Details

- Implement derived scheduling-state propagation/caching across direct and indirect entity references
- Port relationship traversal semantics for scheduled/unscheduled correctness (including group nesting/cycle tolerance) from `schedule-core/src/data/post_process.rs`
- Complete field validation system in schedule-macro:
  - Currently parses `#[validate]` attribute but does not generate `CheckedField` implementations
  - Generate default validation logic for common field types (required strings non-empty, etc.)
  - Add support for custom validation closures
  - Time-related validation: time range consistency, duration validation
  - Cross-field validation (e.g., panel time vs room availability)
- Integration with `EntityType::validate()` method
- Validate export behavior excludes inactive entries where required

## Acceptance Criteria

- Scheduled/unscheduled state derived correctly from time_range and relationships
- Presenter group nesting handled with cycle tolerance
- `#[validate]` attribute generates working `CheckedField` implementations
- Custom validation closures supported
- Compatibility tests against representative schedule-core behavior
