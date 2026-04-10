# cosam-editor: GUI Framework Selection and Scaffold

## Summary

Select the GUI framework for cosam-editor and create the application scaffold.

## Status

Open

## Priority

Low

## Description

Evaluate and select between GUI framework candidates, then create the initial
application structure.

### Framework Candidates

- **iced** — Pure Rust, Elm-inspired, cross-platform. Good for custom UIs.
  Mature widget set, supports custom rendering.
- **GPUI** — Zed's framework. High performance, good text handling. macOS-first
  but cross-platform support improving. Used in prior cosam-editor prototype.
- Other options to consider: **egui** (immediate mode), **Tauri** (web-based UI)

### Evaluation Criteria

- Cross-platform support (macOS primary, Windows secondary)
- Accessibility support
- Custom rendering for schedule grid
- Keyboard shortcut handling
- Native menu bar support
- Binary size and startup time
- Community and maintenance health

### Scaffold

Once selected, create:

- Application entry point with window setup
- Menu bar skeleton (File, Edit, View, Help)
- Keyboard shortcut mapping
- Empty schedule view placeholder
- File open/save dialogs

## Acceptance Criteria

- Framework decision documented with rationale
- Application compiles and shows a window
- Menu bar and keyboard shortcuts are wired up
- File dialogs work for open/save
