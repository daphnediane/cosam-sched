/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Room signs layout builder (stub — to be implemented).

use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::LayoutConfig;
use crate::model::ScheduleData;

/// Generate Typst source for room door signs.
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
) -> Vec<(String, String)> {
    let _ = (data, brand, config, color_mode);
    vec![]
}
