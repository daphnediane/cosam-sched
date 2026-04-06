# Editor Settings and Preferences

## Summary

Implement functional settings window with export preferences and application configuration.

## Status

Not Started

## Priority

Medium

## Description

Add a settings system for the editor, including export preferences (minification, file paths, templates), application preferences (theme, auto-save), and settings persistence. Infrastructure already exists in `settings.rs` and `ui/settings_window.rs`.

## Implementation Details

- Enable Settings menu item and connect AppSettings action to functional window
- Export settings: minification toggle, custom CSS/JS paths, template paths, default formats
- Application settings: theme preferences, auto-save, window behavior
- Settings persistence to config directory
- Settings loaded on application start
- Validation and error handling for invalid settings

## Acceptance Criteria

- Settings menu item is enabled and functional
- Settings window opens with proper UI layout
- Export preferences work and persist across sessions
- Settings are loaded on application start
- Window can be opened/closed properly
