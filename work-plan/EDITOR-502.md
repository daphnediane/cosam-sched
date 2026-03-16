# Event Editing UI

## Summary

Implement inline editing of individual schedule events.

## Status

Open

## Priority

High

## Description

Allow users to click on an event card to edit its properties: name, description, time, room assignment, panel type, presenters, and flags. Changes should update the in-memory schedule model and mark the file as dirty.

## Implementation Details

- Event detail panel or modal dialog on click
- Editable fields for all event properties
- Time picker for start/end times with duration auto-calculation
- Room and panel type dropdowns populated from schedule data
- Presenter management (add/remove from event)
- Dirty state tracking with unsaved changes indicator
