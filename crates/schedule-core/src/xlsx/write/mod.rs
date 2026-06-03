/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! XLSX export implementation (FEATURE-029).

pub mod common;
mod export;

pub use export::{build_grid_spreadsheet, build_spreadsheet, export_xlsx, export_xlsx_grid};
