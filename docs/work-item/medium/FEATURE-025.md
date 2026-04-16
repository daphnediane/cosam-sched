# Internal Schedule File Format

## Summary

Define and implement the native save/load format for schedule documents.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-024: Change tracking and merge operations

## Description

The internal format is used for saving and loading schedule state, including
CRDT history for sync support.

## Acceptance Criteria

- Save and load round-trips all entity data
- CRDT history is preserved in saved files
- Format is versioned for future compatibility
