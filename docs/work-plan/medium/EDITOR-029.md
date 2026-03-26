# Implement Settings Window and Preferences

## Summary

Add functional settings window with export preferences and application configuration options.

## Status

Open

## Priority

Medium

## Description

Implement a complete settings system for the Cosam Editor application, including:

* Settings window accessible from Edit menu
* Export preferences (minification, file paths, templates)
* Application preferences (theme, shortcuts, etc.)
* Settings persistence using existing settings infrastructure
* Proper integration with GPUI window system

## Implementation Details

### Core Components

* Enable Settings menu item in Edit menu
* Connect AppSettings action to functional settings window
* Use existing SettingsWindow component in ui/settings_window.rs
* Integrate with settings.rs persistence system

### Settings Categories

1. **Export Settings**
   * Minification toggle for HTML exports
   * Custom CSS/JS file paths
   * Template file paths
   * Default export formats

2. **Application Settings**
   * Theme preferences
   * Auto-save settings
   * Window behavior

3. **Advanced Settings**
   * Debug options
   * Performance settings

### Technical Requirements

* Settings window should open as modal or separate window
* Settings should persist to config directory
* Changes should apply immediately or with confirmation
* Handle settings validation and error cases

## Acceptance Criteria

* [ ] Settings menu item is enabled and functional
* [ ] Settings window opens with proper UI layout
* [ ] Export preferences work and persist
* [ ] Settings are loaded on application start
* [ ] Window can be opened/closed properly
* [ ] Settings validation works correctly

## Notes

* Settings infrastructure already exists in settings.rs
* SettingsWindow UI component is already implemented
* Need to connect menu action to window creation
* Consider using existing SettingsPlaceholder as starting point
