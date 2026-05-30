/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared Typst block generators for print layouts.
//!
//! - [`banner`] — compact brand-color page header (repeats on every page)
//! - [`panels`] — panel description blocks, time-grouped rendering

pub(crate) mod banner;
pub(crate) mod panels;
