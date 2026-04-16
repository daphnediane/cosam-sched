# Widget Display JSON Export

## Summary

Implement export of schedule data to the JSON format consumed by the calendar display widget.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-019: Schedule container + EntityStorage

## Description

The calendar widget renders schedule data from a JSON file. This work item
defines and implements the export format (clean break from v9/v10 format).

## Acceptance Criteria

- Export produces valid JSON matching widget schema
- All scheduled panels with times and rooms are included
- Presenter names are correctly formatted
