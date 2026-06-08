# META-117: cosam-viewer — cross-platform schedule viewer

## Summary

Tracker for all cosam-viewer work: initial viewer app and deferred enhancements.

## Status

Blocked

## Priority

Medium

## Description

cosam-viewer is a Dioxus 0.7 app that reads the cosam widget JSON format and
renders a schedule UI similar to the JS widget, targeting macOS, iPadOS, and Android.

## Work Items

- FEATURE-116: Initial cosam-viewer app (list view, filters, day tabs, detail modal, 4 themes)
- FEATURE-118: Grid view (rooms × time slots)
- FEATURE-119: My Schedule / bookmarking
- FEATURE-120: Mobile-specific build and deploy configuration

## Notes

FEATURE-116 is the initial scaffold commit. FEATURE-118, -119, and -120 are deferred
enhancements that are not blocked by each other.
