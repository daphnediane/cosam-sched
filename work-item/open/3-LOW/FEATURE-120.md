# FEATURE-120: cosam-viewer — mobile build and deploy configuration

## Summary

Configure `dx` build targets for Android and iPadOS, including app metadata,
icons, and CI/CD pipeline integration.

## Status

Open

## Priority

Low

## Description

Set up the `Dioxus.toml`, Android manifest, iOS Info.plist, and icon assets
needed to produce release builds of cosam-viewer for Android and iPadOS via
`dx build --platform android` and `dx build --platform ios`.

## Implementation Details

- Add `apps/cosam-viewer/Dioxus.toml` with app name, bundle ID, and platform targets
- Add `assets/` directory with app icons (1024×1024 + platform-specific sizes)
- Add `AndroidManifest.xml` and `Info.plist` stubs
- Document build steps in `docs/cosam-viewer-build.md`
- Investigate file-picker alternative for mobile (e.g., `rfd` mobile support or
  platform file-provider intent)
- Optional: add GitHub Actions workflow for iOS/Android builds

## Acceptance Criteria

- `dx build --platform android` produces a signed APK
- `dx build --platform ios` produces an IPA (or simulator build)
- App icon and display name correct on both platforms
- File open works on each platform (opens widget JSON from device storage)
