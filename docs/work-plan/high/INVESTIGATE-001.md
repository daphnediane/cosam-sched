# Investigate xlsx_update functionality and corruption issues

## Summary

Investigate xlsx_update module for potential corruption issues and determine if it should be disabled

## Status

Open

## Priority

High

## Description

The xlsx_update module may be creating corrupted Excel files (though openable). Need to investigate:

- Whether xlsx_update is actually corrupting files
- If the corruption is cosmetic or functional
- Whether the module should be disabled until properly fixed
- The fundamental design issues with update vs export approaches

## Current Issues

- test_post_save_cleanup_removes_deleted is failing
- Questions about whether staff presenters should get individual columns vs "Other" column
- Potential mismatch between update logic and expected presenter column behavior

## Acceptance Criteria

- Determine root cause of Excel file corruption
- Decide whether to disable xlsx_update temporarily
- Fix or document the presenter column threshold logic
- Ensure all xlsx_update tests pass or are properly disabled

## Notes

xlsx_update uses existing spreadsheet columns rather than building new ones like xlsx_export. This may cause issues when presenter assignments change between exports.
