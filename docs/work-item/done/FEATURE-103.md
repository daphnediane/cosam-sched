# FEATURE-103: Field Comparison Across Codebase Versions

## Summary

Compare and document the field definitions between the current main branch, v9, v10-try1, and v10-try3 to identify gaps and ensure complete coverage.

## Status

Completed

## Priority

Medium

## Description

Investigate the field definitions across different versions of the codebase to understand:

- Which fields exist in v9 that are missing in main
- Which fields were added in v10-try1 but not carried forward to main
- Which fields exist in v10-try3 that should be considered for main
- Field naming and type differences between versions
- Deprecated or renamed fields

This investigation will help ensure that the main branch has complete field coverage for when binary formats become the primary storage format.

## Acceptance Criteria

- Document listing all fields in v9, v10-try1, v10-try3, and main
- Identify gaps where main is missing fields present in other versions
- Document field type/naming differences
- Recommend which missing fields should be added to main for completeness
