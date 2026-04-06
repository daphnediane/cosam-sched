# Embedded Webview Preview

## Summary

Revisit embedding a webview directly in the editor window once gpui_web is available.

## Status

Not Started

## Priority

Low

## Description

The editor currently opens schedule previews in the system browser using a temporary HTML file with auto-reload polling. Once `gpui_web` becomes available, embed the preview directly inside the editor for side-by-side editing.

## Implementation Details

- Monitor gpui_web availability in the GPUI crate
- Evaluate whether gpui_web supports enough HTML/CSS/JS for the cosam-calendar widget
- Replace or supplement system browser preview with embedded webview panel
- Consider live-updating the embedded preview on schedule changes without file polling
- Fall back to system browser if gpui_web has rendering limitations

## Acceptance Criteria

- Preview renders inside the editor window
- Live updates on schedule changes without manual refresh
- Fallback to system browser if webview insufficient
