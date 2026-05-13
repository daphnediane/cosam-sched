# FEATURE-115: Separate Timeline Sheet in XLSX

## Summary

Separate Timeline Sheet in XLSX

## Status

Open

## Priority

Medium

## See also

- FEATURE-083: Separate Hotel Room sheet in XLSX import/export

## Description

Add a dedicated Timeline sheet to the XLSX format to separate timeline events from regular panels. This aligns with the new Timeline entity type and simplifies the data model.

## Motivation

Currently timeline events are stored as Panel entities with `is_timeline` panel type flag. The widget export filters these out separately. With the new Timeline entity type, we can:

* Store timelines as their own entity type
* Have a dedicated Timeline sheet in XLSX
* Simplify panel-related code that currently filters for/against timelines
* Make the data model more explicit and type-safe

## Requirements

* XLSX import reads Timeline sheet and creates Timeline entities
* XLSX export writes Timeline entities to a dedicated Timeline sheet
* Timeline sheet includes: code, name, description, note, time, panel_types
* Widget export uses Timeline entities directly instead of filtering panels
* Update panel-related code to leverage timeline separation where applicable

## Implementation Notes

* Timeline sheet should follow similar structure to Schedule sheet but simplified (no presenter columns, no room assignments)
* Timeline entities have a single time point (not a duration range)
* Panel types can be associated with timelines via the HALF_EDGE_PANEL_TYPES edge
* Import should read Timeline sheet before Schedule sheet for proper panel type resolution
