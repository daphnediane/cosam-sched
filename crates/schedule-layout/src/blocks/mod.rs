/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared Typst block generators for print layouts.
//!
//! - [`banner`] — compact brand-color page header (repeats on every page)
//! - [`grid`] — schedule grid rendering (CSS-grid-style Typst grid)
//! - [`panels`] — panel description blocks, time-grouped rendering

pub(crate) mod banner;
pub(crate) mod grid;
pub(crate) mod panels;
