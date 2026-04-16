# cosam-editor: GUI Framework Selection and Scaffold

## Summary

Select the GUI framework for cosam-editor and create the application scaffold.

## Status

Open

## Priority

Low

## Blocked By

- FEATURE-021: Edit command system with undo/redo

## Description

Evaluate and select between GUI framework candidates, then create the initial
application structure.

### Framework candidates

- **iced** — Pure Rust, Elm-inspired, cross-platform
- **GPUI** — Zed's framework, high performance, macOS-first
- **egui** — immediate mode, easy prototyping
- **Tauri** — web-based UI

### Scaffold

- Application entry point with window setup
- Menu bar skeleton (File, Edit, View, Help)
- Keyboard shortcut mapping
- File open/save dialogs

## Acceptance Criteria

- Framework decision documented with rationale
- Application compiles and shows a window
- Menu bar and keyboard shortcuts wired up
- File dialogs work for open/save
