# Embedded Webview Preview

## Summary

Revisit embedding a webview directly in the editor window once gpui_web is available.

## Status

Open

## Priority

Low

## Description

The editor currently opens schedule previews in the system browser using a temporary HTML file with auto-reload polling. This works but requires context-switching between the editor and browser windows.

Once `gpui_web` (GPUI's planned web/webview integration) becomes available, revisit embedding the preview directly inside the editor window. This would allow side-by-side editing and preview without leaving the application.

## Implementation Details

- Monitor gpui_web availability in the GPUI crate
- Evaluate whether gpui_web supports enough HTML/CSS/JS for the cosam-calendar widget
- Replace or supplement the system browser preview with an embedded webview panel
- Consider live-updating the embedded preview on schedule changes without file polling
- Fall back to system browser if gpui_web has rendering limitations
