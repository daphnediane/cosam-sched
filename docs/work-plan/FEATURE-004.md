# Develop a standalone editor app

## Summary

Create a cross-platform desktop application for schedule editing.

## Status

In Progress

## Priority

Low

## Description

Build a standalone cross-platform desktop editor using Rust and GPUI for editing schedules and generating output. Supports macOS, Windows, and Linux.

## Implementation Details

- Framework: Rust with GPUI for cross-platform UI
- Data model: Rust structs matching schedule.json schema (serde)
- JSON load/display with day tabs, room sidebar, event cards
- Future: XLSX import/export, event editing, conflict detection
- Future: Google Sheets and OneDrive integration
- Package as executable for all platforms
