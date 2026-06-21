/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule data model for layout generation.
//!
//! The widget/interchange DTO lives in the [`schedule_widget_format`] leaf crate
//! and is shared verbatim by every consumer. Layout consumes it directly under
//! the historical aliases below (`ScheduleData`, `Panel`, …); there is no longer
//! a separate mirror type or field-by-field mapper to drift.
//!
//! Build it from a [`schedule_core::schedule::Schedule`] via [`from_schedule`],
//! or deserialize from a widget-JSON file via [`ScheduleData::load`] /
//! [`ScheduleData::from_json`] for standalone use.

pub use schedule_widget_format::{
    WidgetDaySpan as DaySpan, WidgetExport as ScheduleData, WidgetMeta as Meta,
    WidgetPanel as Panel, WidgetPanelColors as PanelTypeColors, WidgetPanelType as PanelType,
    WidgetPresenter as Presenter, WidgetRoom as Room, WidgetTimeline as TimelineEntry,
};

use schedule_core::value::timezone::epoch_to_local_iso;
use schedule_core::widget_json::{export_to_widget_json, WidgetJsonError};

// ── FEATURE-154 time helpers ────────────────────────────────────────────────
//
// Times in the DTO are canonical Unix epoch seconds. The layout engine works in
// local wall-clock ISO strings (date bucketing, slot keys, time-of-day labels),
// so these helpers recover that string from the epoch using the schedule's
// timezone (`meta.timezone`). An empty/unknown zone is treated as UTC.

/// Local wall-clock ISO (`YYYY-MM-DDTHH:MM:SS`) for a panel's start, in `tz`.
#[must_use]
pub fn panel_start_iso(p: &Panel, tz: &str) -> Option<String> {
    p.start_epoch.map(|e| epoch_to_local_iso(e, tz))
}

/// Local wall-clock ISO for a panel's end, in `tz`.
#[must_use]
pub fn panel_end_iso(p: &Panel, tz: &str) -> Option<String> {
    p.end_epoch.map(|e| epoch_to_local_iso(e, tz))
}

/// Local wall-clock ISO for a timeline entry's start, in `tz`.
#[must_use]
pub fn timeline_start_iso(t: &TimelineEntry, tz: &str) -> Option<String> {
    t.start_epoch.map(|e| epoch_to_local_iso(e, tz))
}

/// Schedule-window start as a local wall-clock ISO string; `None` when absent.
#[must_use]
pub fn meta_start_iso(m: &Meta) -> Option<String> {
    (m.start_epoch != 0).then(|| epoch_to_local_iso(m.start_epoch, &m.timezone))
}

/// Schedule-window end as a local wall-clock ISO string; `None` when absent.
#[must_use]
pub fn meta_end_iso(m: &Meta) -> Option<String> {
    (m.end_epoch != 0).then(|| epoch_to_local_iso(m.end_epoch, &m.timezone))
}

/// Build layout data directly from a [`schedule_core::schedule::Schedule`].
///
/// When `private` is false this uses the public-export view (no private panels,
/// timeline entries, or unlisted presenters). When `private` is true it includes
/// private panels and surfaces unlisted (uncredited) presenters on their panels,
/// so per-presenter sections can attribute unlisted guests. Break synthesis runs
/// over whichever panel set is visible, so each visibility level is internally
/// consistent.
///
/// This is a thin wrapper over [`export_to_widget_json`]: the layout engine and
/// the widget now share one type, so no conversion happens here.
pub fn from_schedule(
    schedule: &schedule_core::schedule::Schedule,
    title: &str,
    private: bool,
) -> Result<ScheduleData, WidgetJsonError> {
    export_to_widget_json(schedule, title, private)
}
