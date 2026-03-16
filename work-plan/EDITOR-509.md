# Application Packaging and Distribution

## Summary

Package the editor as standalone executables for macOS, Windows, and Linux.

## Status

Open

## Priority

Medium

## Description

Set up build and packaging pipelines to produce distributable application bundles. Users should be able to download and run the editor without installing Rust or other development tools.

## Implementation Details

- macOS: `.app` bundle with code signing
- Windows: `.exe` with optional installer (MSI or NSIS)
- Linux: AppImage or Flatpak
- CI/CD pipeline for automated builds on tag/release
- Application icon and metadata
- Auto-update mechanism (check for new versions on launch)
- Include sample data for first-run experience
