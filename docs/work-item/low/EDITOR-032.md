# cosam-editor: GUI Framework Selection and Scaffold

## Summary

Select the GUI framework for cosam-editor and create the application scaffold.

## Status

In Progress

## Priority

Low

## Blocked By

- FEATURE-021: Edit command system with undo/redo (Completed)

## Description

Evaluate and select between GUI framework candidates, then create the initial
application structure.

### Framework evaluation

Two parallel scaffolds were built for side-by-side comparison:

- **`apps/cosam-editor-gpui/`** — GPUI 0.2 scaffold (ported from v9 prototype)
- **`apps/cosam-editor-dioxus/`** — Dioxus 0.7 scaffold (new implementation)

Note: GPUI and Dioxus have an incompatible `cocoa` crate version conflict
(`gpui=0.2` pins `cocoa=0.26.0`; `dioxus-desktop=0.7` requires `cocoa>=0.26.1`).
Both editor apps are standalone workspaces, excluded from the root workspace.
Build with:

```sh
cargo build --manifest-path apps/cosam-editor-gpui/Cargo.toml
cargo build --manifest-path apps/cosam-editor-dioxus/Cargo.toml
```

### Framework candidates

- **iced** — Pure Rust, Elm-inspired, cross-platform (desktop only, no mobile)
- **GPUI** — Zed's framework, high performance, macOS-first; has v9 reference prototype
- **egui** — immediate mode, easy prototyping (desktop only)
- **Tauri v2** — web frontend + Rust backend; first-class iOS/Android support
- **Dioxus 0.7** — pure Rust with React-like components, WebView-based, iOS/Android support

### Scaffold (both apps implement)

- Application entry point with window setup (1200×800)
- Menu bar skeleton: File, Edit, View, Help
- Keyboard shortcuts: Cmd-O, Cmd-S, Cmd-Shift-S, Cmd-Z, Cmd-Shift-Z, Cmd-W, Cmd-Q
- File open dialog (`.cosam` and `.xlsx` via `rfd`)
- Placeholder body showing schedule UUID + panel count when a file is loaded
- Undo/redo wired to `EditContext::undo()` / `redo()`
- Save/export stubs (deferred to EDITOR-033)

## Acceptance Criteria

- [x] Framework candidates documented with rationale
- [x] Both apps compile
- [x] Both show a window with header and placeholder body
- [x] Menu bars and keyboard shortcuts wired up
- [x] File open dialogs work (load `.cosam` and `.xlsx`)
- [ ] Framework decision made and documented (pending evaluation)
- [ ] Winner renamed/promoted to `apps/cosam-editor`
