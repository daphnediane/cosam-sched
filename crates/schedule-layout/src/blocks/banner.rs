/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Compact brand-colored page header for description and workshop listings.

use crate::brand::BrandConfig;
use crate::typst_gen::escape_typst;

/// Generate a `#set page(header: …)` Typst directive that renders a compact
/// brand-primary bar (logo + `heading` text) on every page.
///
/// Must be emitted after `preamble()` so that `brand-primary` is already
/// defined in the document scope.
pub(crate) fn page_header(brand: &BrandConfig, heading: &str) -> String {
    let inner = if let Some(path) = brand.meta.logo_path.as_ref().and_then(|p| p.to_str()) {
        format!(
            "#grid(columns: (auto, 1fr), align: (left + horizon, right + horizon), \
             image(\"{path}\", height: 0.3in), \
             [#text(fill: white, weight: \"bold\", size: 11pt)[{heading}]])",
            path = path.replace('\\', "/"),
            heading = escape_typst(heading),
        )
    } else {
        format!(
            "#text(fill: white, weight: \"bold\", size: 11pt)[{heading}]",
            heading = escape_typst(heading),
        )
    };
    format!(
        "#set page(header: block(fill: brand-primary, width: 100%, \
         inset: (x: 10pt, y: 5pt))[\n  {inner}\n])\n",
        inner = inner,
    )
}
