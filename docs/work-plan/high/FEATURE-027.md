# Support Google Sheets for schedule data

## Summary

Enable reading schedule data directly from Google Sheets.

## Status

Open

## Priority

High

## Description

The convention is moving to Google Sheets next year. The converter needs to support reading from Google Sheets API in addition to XLSX files.

## Implementation Details

- Research Google Sheets API integration options
- Decide between server-side conversion or client-side
- Add OAuth2 authentication for private sheets
- Maintain backward compatibility with XLSX
- Consider caching for performance
