# Fix missing presenters

## Summary

Converting the existing spreadsheets loses presenter information during the conversion process.

## Status

Completed

## Priority

High

## Description

The converter is not properly extracting presenter data from the spreadsheet columns. This results in events without presenter information in the generated JSON, which is critical for attendees to know who is running each event.

## Steps to Fix

1. ~~Identify the presenter column in the spreadsheet format~~

2. ~~Update Events.pm to properly parse presenter data~~

3. ~~Handle multiple presenters if applicable~~

4. Test with existing spreadsheets to ensure data is preserved

## Resolution

The converter only supported `g1`/`Guest1`-style presenter column headers.
Actual spreadsheets use the `Kind:Name=Group` format from schedule-to-html
(e.g. `G:Yaya Han`, `P:Other`). Updated `_parse_presenter_header` in
Events.pm to detect this format and rewrote the extraction loop to handle
both named-header columns (cell = flag) and value-in-cell columns.
Also added a fallback for a generic `Presenter`/`Presenters` column.
Spreadsheet format documentation added to `docs/spreadsheet-format.md`.
