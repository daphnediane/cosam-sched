/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared widget/interchange JSON format.
//!
//! This is the single serde DTO for the public-facing widget display format
//! documented in `docs/widget-json-format.md`. It is produced by
//! `schedule-core` (export) and consumed directly by `schedule-layout`, the
//! WASM print plugin, `cosam-convert`, `cosam-viewer`, and the JavaScript
//! widget. Keeping one set of structs here means every consumer parses exactly
//! the shape the exporter emits — no per-consumer mirror types to drift.
//!
//! The crate depends only on `serde`/`serde_json`/`thiserror` so it can be
//! linked into a size-optimized WASM build with no heavier transitive deps.

mod config;
mod schedule;

pub use config::{
    ScheduleBrand, ScheduleBrandColors, ScheduleBrandMeta, ScheduleConfig, SchedulePrintFont,
    SchedulePrintFontSizes, SchedulePrintFonts, SchedulePrintFormat,
};
pub use schedule::{
    WidgetExport, WidgetMeta, WidgetPanel, WidgetPanelColors, WidgetPanelType, WidgetPresenter,
    WidgetRoom, WidgetTimeline,
};

use thiserror::Error;

/// Errors from loading/parsing widget JSON.
#[derive(Debug, Error)]
pub enum WidgetFormatError {
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
