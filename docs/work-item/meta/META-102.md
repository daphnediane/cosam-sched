# META-102: Storage and XLSX Round-Trip Infrastructure

## Summary

Implement sidecar storage for provenance and extra metadata, and enable in-place XLSX updates.

## Status

Open

## Priority

High

## Description

This meta tracks work on storage infrastructure needed for XLSX round-trip workflows:

- SourceInfo sidecar to track entity origins (file, sheet, row)
- Extra metadata sidecar for unknown XLSX columns
- In-place XLSX update to preserve formatting and custom content

## Work Items

- FEATURE-081: Import Provenance / SourceInfo Sidecar
- FEATURE-082: Extended Entity Metadata (Unknown XLSX Columns)
- FEATURE-084: XLSX Spreadsheet Update (In-Place Save)
- FEATURE-103: Field Comparison Across Codebase Versions

## Notes

FEATURE-081 and FEATURE-082 are prerequisites for FEATURE-084.
