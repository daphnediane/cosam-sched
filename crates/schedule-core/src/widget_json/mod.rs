/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON format structures and I/O.
//!
//! This module provides the widget JSON display format structures documented in
//! `docs/widget-json-format.md`, along with export and import functionality.

mod export;
mod import;
mod types;

// Re-export public types
pub use types::{
    WidgetExport, WidgetMeta, WidgetPanel, WidgetPanelColors, WidgetPanelType, WidgetPresenter,
    WidgetRoom, WidgetTimeline,
};

// Re-export public functions from export module
pub use export::{export_to_widget_json, save_to_file, save_to_json};

// Re-export public functions from import module
pub use import::{import_from_widget_json, load_from_file, load_from_json, load_from_url};

// Re-export error type
pub use export::WidgetJsonError;
