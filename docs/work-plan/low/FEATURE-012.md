# Google Sheets Integration

## Summary

Enable reading schedule data directly from Google Sheets API.

## Status

Not Started

## Priority

Low

## Description

The convention is moving to Google Sheets. The converter and editor need to support reading from Google Sheets API in addition to XLSX files. This is transport/authentication only — multi-device sync is a separate concern.

## Implementation Details

- Google Sheets API integration via OAuth 2.0 (desktop app flow)
- Read schedule data from a configured Google Sheet (table/sheet names matching XLSX path)
- Write changes back to Google Sheets with the same semantic output as local conversion/export
- Handle API rate limits and connectivity issues with actionable user-facing errors
- Cross-platform credential storage (macOS Keychain, Windows Credential Manager)
- Support direct Google Sheets URLs and robust spreadsheet ID extraction
- Validate auth, permissions, and error-path UX before calling production-ready
- Add a validation matrix covering real-world sheets (named tables, no tables, mixed formatting)
- Legacy notes archived in branch `feature/final-perl-converter`

## Acceptance Criteria

- Schedule data can be read from Google Sheets
- Authentication flow works on macOS and Windows
- Data round-trips correctly through Google Sheets read/write
- Error handling covers rate limits, auth failures, and connectivity issues
- Backward compatibility with XLSX workflow maintained
