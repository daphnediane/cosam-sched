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
//! - [`grid`] — time-grid layout computation (time slots, room columns, cell spans)
//! - [`typst_gen`] — Typst `.typ` source generation
//! - [`formats`] — per-format layout builders (schedule, workshop poster, etc.)

pub mod brand;
pub mod color;
pub mod formats;
pub mod grid;
pub mod model;
pub mod typst_gen;

pub use brand::BrandConfig;
pub use color::{ColorMode, PanelColor};
pub use grid::{GridLayout, LayoutConfig, LayoutFilter, LayoutFormat, PaperSize, SplitMode};
pub use model::ScheduleData;
