# FEATURE-155: Kiosk mode + live current-panel highlighting

## Summary

Bring the old schedule-to-html kiosk display into the widget as a plugin, and
highlight currently-running panels in all views.

## Status

Completed

## Priority

Medium

## Description

The legacy `schedule-to-html/kiosk/` shipped a standalone kiosk: a banner with a
live clock, a top pane with the auto-scrolling schedule grid (active timeslot
highlighted), and a bottom pane with the current + upcoming panel per room. The
modern widget has no equivalent and no notion of "what's running now."

Add a **kiosk plugin** (same extension mechanism as the advanced print format)
plus a live current-panel highlight that also works in the normal grid/list
views. "Now" is driven by the schedule timezone; the banner clock shows venue
wall-clock. Auto-scroll resumes 2 minutes after manual interaction (immediately
when the clock is clicked).

## Implementation Details

- **Core (`widget/cosam-calendar.js`)**: replace single-plugin wiring with a
  `state._plugins` array and one canonical host object `_pluginHost()` (superset
  of the old `_printPluginCtx()`, adds `helpers`, `nowEpoch()`,
  `refreshCurrent()`). Add the current-panel engine: `_currentNowEpoch()`,
  dataset stamps (`startEpoch`/`endEpoch`/`slot`) on event elements + time
  headers, `_applyCurrentHighlight()` toggling `.cosam-current` on a ~30s timer.
  Optional `nowProvider` / `?cosamNow=` debug hook.
- **`widget/cosam-calendar.css`**: `.cosam-current` styling for grid + list, with
  a non-color cue (accent ring + "Now" marker) for WCAG.
- **`widget/print-format-advanced.js`**: migrate to the unified host (mechanical).
- **`widget/kiosk.js` + `widget/kiosk.css`** (new): `KioskPlugin` adds a Kiosk
  toolbar button; full-viewport banner / grid pane / detail pane. Self-registers
  into `window.CosAmCalendarPlugins`.
- **`widget/build.mjs`**: bundle `kiosk.min.{js,css}`.
- **`apps/cosam-convert/src/{embed.rs,main.rs}`**: general repeatable `--plugin
  <name|file>` CLI; inline plugin JS/CSS; init passes
  `plugins: (window.CosAmCalendarPlugins || [])`.

## Acceptance Criteria

- Kiosk button enters a full-screen kiosk: ticking venue-tz clock, auto-scrolling
  grid with current timeslot + running panels highlighted, per-room
  current/upcoming bottom pane. Esc/× exits.
- Manual scroll/click pauses auto-scroll for 2 min; clock click resumes now.
- Clicking a grid event opens its detail modal over the kiosk.
- Normal grid/list views highlight currently-running panels.
- `cosam-convert --plugin kiosk` inlines the plugin into generated pages.

## Notes

Delivered beyond the original scope during review:

- Fully theme-driven kiosk chrome + highlights (light/dark/high-contrast). Active
  hour reads across the window via an accent wash on empty cells + time column;
  running panels tint to their panel-type hue at theme-pinned OKLCh L/C
  (`--cosam-current-l` / `--cosam-current-c`).
- Crystal-ball **future preview**: toggle a previewed time (datetime field or
  click a time-column slot); drives the shared clock so grid/detail/clock all
  follow it. Exits to live time.
- Per-room **countdown** ("in 45m" / "in 1h 23m") on upcoming-panel cells.
- Clicking a Current/Upcoming cell opens the shared detail modal (above the
  kiosk), matching a grid-panel click.
- Test/deploy hooks: `?cosamNow=<epoch|ISO>` freezes "now", `?cosamTheme=<name>`
  forces a theme, `#kiosk` auto-opens the kiosk.
- Wired `--plugin kiosk` into `scripts/{export-schedules.sh,export-schedules.ps1,
  sync-schedule.sh,sync-schedule.ps1}`.
