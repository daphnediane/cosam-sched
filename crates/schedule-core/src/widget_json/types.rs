/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON format structures.
//!
//! The structs themselves now live in the dependency-light
//! [`schedule_widget_format`] leaf crate so every consumer (export, layout, the
//! WASM print plugin, the JS widget) shares one definition. They are re-exported
//! here to preserve the historical `schedule_core::widget_json::*` paths and are
//! documented in `docs/widget-json-format.md`.

pub use schedule_widget_format::{
    WidgetExport, WidgetMeta, WidgetPanel, WidgetPanelColors, WidgetPanelType, WidgetPresenter,
    WidgetRoom, WidgetTimeline,
};
