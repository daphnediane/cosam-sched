/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

pub mod app;
pub mod grid;

pub use app::App;

// ---------------------------------------------------------------------------
// Shared grid layout constants
// ---------------------------------------------------------------------------

/// Width of the sticky time-label column (first grid column), in CSS pixels.
/// Must match the value used in the `grid-template-columns` inline style in
/// [`grid::GridView`].
pub const GRID_TIME_COL_PX: f64 = 64.0;

/// Minimum useful room column width, in CSS pixels (≈ 5 characters at
/// 0.8125 rem).  Below this threshold the grid becomes unreadable and the
/// view falls back to list mode.
pub const GRID_MIN_ROOM_COL_PX: f64 = 60.0;

/// Height of the sticky room-header row (row 1), in CSS pixels.
pub const GRID_HEADER_ROW_PX: u32 = 36;

/// Height of each 30-minute time-slot row (rows 2+), in CSS pixels.
pub const GRID_SLOT_ROW_PX: u32 = 60;
