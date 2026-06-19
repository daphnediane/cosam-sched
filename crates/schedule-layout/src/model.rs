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
    WidgetExport as ScheduleData, WidgetMeta as Meta, WidgetPanel as Panel,
    WidgetPanelColors as PanelTypeColors, WidgetPanelType as PanelType,
    WidgetPresenter as Presenter, WidgetRoom as Room, WidgetTimeline as TimelineEntry,
};

use schedule_core::widget_json::{export_to_widget_json, WidgetJsonError};

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
