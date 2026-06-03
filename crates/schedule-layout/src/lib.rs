/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Layout engine, brand config, and Typst source generation for cosam-sched print outputs.
//!
//! # Modules
//!
//! - [`model`] — widget JSON data model (deserialization)
//! - [`brand`] — brand configuration (colors, fonts, logo, site URL)
//! - [`color`] — color mode, panel type color resolution, grayscale fallback
//! - [`config`] — layout configuration: paper, orientation, content/split modes
//! - [`geometry`] — page/banner/footer dimension constants and Typst `#let` emitter
//! - [`fonts`] — font sizes, typeface specs, and the Typst `#let` font emitter
//! - [`timegrid`] — time-grid layout computation (time slots, room columns, cell spans)
//! - [`typst_gen`] — Typst `.typ` source generation
//! - [`document`] — the unified multi-section layout builder

pub mod blocks;
pub mod brand;
pub mod color;
pub mod config;
pub mod document;
pub mod fonts;
pub mod geometry;
pub mod model;
pub mod time_fmt;
pub mod timegrid;
pub mod typst_gen;

pub use brand::BrandConfig;
pub use color::{ColorMode, PanelColor};
pub use config::{
    ContentMode, FooterMode, LayoutConfig, Orientation, PanelFilter, PaperSize, SectionSplit,
    TimeSplit,
};
pub use model::ScheduleData;
pub use timegrid::GridLayout;
