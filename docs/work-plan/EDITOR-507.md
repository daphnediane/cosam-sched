# Google Sheets Integration

## Summary

Support reading from and writing to Google Sheets.

## Status

Open

## Priority

Low

## Description

Enable the editor and converter CLI to read and write schedule data via Google Sheets, while keeping this item focused on transport/authentication and schema parity rather than multi-device sync strategy.

Current state: the Perl converter has an unverified Google Sheets path and has not been production-tested for this workflow. Rust support should include explicit validation against real sheets before considering this complete.

Legacy implementation notes from the removed Perl-era docs are archived in branch `feature/final-perl-converter` (`GOOGLE_SHEETS.md`, `google-sheets-config.example.yaml`). Key takeaways to carry forward for Rust:

- OAuth 2.0 credentials flow with explicit token-file handling
- Support direct Google Sheets URLs and robust spreadsheet ID extraction
- Handle both formal table metadata and heuristic range detection
- Validate auth, permissions, and error-path UX before calling the feature production-ready

## Implementation Details

- Google Sheets API integration via OAuth 2.0 (desktop app flow)
- Read schedule data from a configured Google Sheet (table/sheet names matching XLSX path)
- Write changes back to Google Sheets with the same semantic output as local conversion/export
- Handle API rate limits and connectivity issues with actionable user-facing errors
- Cross-platform credential storage:
  - macOS: Keychain
  - Windows: Windows Credential Manager (or equivalent secure store)
- Add a validation matrix covering real-world sheets (named tables present, no tables, mixed formatting)
- Explicitly exclude multi-device sync/conflict resolution from this ticket; track that separately
