/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Branded page-header bar used by all layout formats.

use chrono::{DateTime, Local};

use crate::brand::BrandConfig;
use crate::typst_gen::{build_font_spec, escape_typst};

/// Generate a `#set page(header: …)` Typst directive for all layout formats.
///
/// Logo placement follows these rules (logo is used when `brand.meta.logo_path`
/// is configured):
///
/// | `left`  | `right` | logo placement              |
/// |---------|---------|-----------------------------|
/// | Some    | Some    | center (between the labels) |
/// | Some    | None    | right (opposite left)       |
/// | None    | Some    | left (opposite right)       |
/// | None    | None    | center                      |
///
/// When no logo is configured: both labels are shown; a single label is
/// centered in the bar.
///
/// Text is rendered ALL CAPS in the banner font (falling back to the heading
/// font) at 28 pt.
///
/// Must be emitted after `preamble()` so that `brand-primary` is already
/// defined in the document scope.
pub(crate) fn page_header(brand: &BrandConfig, left: Option<&str>, right: Option<&str>) -> String {
    let logo_path = brand
        .meta
        .logo_path
        .as_ref()
        .and_then(|p| p.to_str())
        .map(|p| p.replace('\\', "/"));

    let inner = build_inner(brand, left, right, logo_path.as_deref());

    format!(
        "#set page(header: block(fill: brand-primary, width: 100%, \
         inset: _banner-inset)[\n  {inner}\n])\n",
    )
}

/// Generate a `#set page(header: …)` directive whose right-hand label is *raw
/// Typst content* (e.g. a `context` expression for a running header) rather than
/// a literal string.
///
/// The logo (when configured) sits on the left and the content is right-aligned;
/// with no logo the content is centered.  Styling matches [`page_header`]
/// (ALL CAPS, banner font, 28 pt).  `right_content` is inserted verbatim, so the
/// caller is responsible for it being valid Typst.
pub(crate) fn page_header_running(brand: &BrandConfig, right_content: &str) -> String {
    let logo_path = brand
        .meta
        .logo_path
        .as_ref()
        .and_then(|p| p.to_str())
        .map(|p| p.replace('\\', "/"));

    let font_spec = build_font_spec(
        brand.fonts.banner_or_default(),
        brand.fonts.banner_style(),
        Some(brand.fonts.banner_weight_or_default()),
    );
    let label = format!(
        "#text(fill: white, size: _banner-text-size, {font_spec})[#upper[{right_content}]]"
    );

    let inner = match logo_path {
        Some(p) => format!(
            "#grid(columns: (auto, 1fr), align: (left + horizon, right + horizon), \
             image(\"{p}\", height: _logo-height), [{label}])",
        ),
        None => format!("#align(center)[{label}]"),
    };

    format!(
        "#set page(header: block(fill: brand-primary, width: 100%, \
         inset: _banner-inset)[\n  {inner}\n])\n",
    )
}

/// Generate a `#set page(header: …)` directive with *two* raw-Typst content
/// slots — a left label and a right label — for running headers that vary per
/// page (e.g. room signs, where each page shows its room on the left and day on
/// the right).
///
/// Layout mirrors [`page_header`]'s both-labels case: with a logo, the slots sit
/// either side of a centered logo (L | logo | R); without one, they split the
/// bar evenly (L | R).  Both contents are styled like the other banners (ALL
/// CAPS, banner font, 28 pt) and inserted verbatim, so the caller is responsible
/// for them being valid Typst (typically `context` expressions).
pub(crate) fn page_header_running_split(
    brand: &BrandConfig,
    left_content: &str,
    right_content: &str,
) -> String {
    let logo_path = brand
        .meta
        .logo_path
        .as_ref()
        .and_then(|p| p.to_str())
        .map(|p| p.replace('\\', "/"));

    let font_spec = build_font_spec(
        brand.fonts.banner_or_default(),
        brand.fonts.banner_style(),
        Some(brand.fonts.banner_weight_or_default()),
    );
    let wrap = |content: &str| {
        format!("#text(fill: white, size: _banner-text-size, {font_spec})[#upper[{content}]]")
    };
    let left = wrap(left_content);
    let right = wrap(right_content);

    let inner = match logo_path {
        Some(p) => format!(
            "#grid(columns: (1fr, auto, 1fr), \
             align: (left + horizon, center + horizon, right + horizon), \
             [{left}], image(\"{p}\", height: _logo-height), [{right}])",
        ),
        None => format!(
            "#grid(columns: (1fr, 1fr), align: (left + horizon, right + horizon), \
             [{left}], [{right}])",
        ),
    };

    format!(
        "#set page(header: block(fill: brand-primary, width: 100%, \
         inset: _banner-inset)[\n  {inner}\n])\n",
    )
}

/// Generate a `#set page(footer: …)` directive showing timestamps, a centered
/// page number, and the organization/site on the right.
///
/// `timestamps` is a pre-formatted string such as
/// `"Modified: Jun 15 4:00 PM | Generated: Jun 15 4:05 PM"` (empty to omit).
/// `site` is the right-hand label (site URL or org name; empty to omit).
///
/// The page number uses Typst's `counter(page)` so it reflects the final page
/// count across the whole document, including blank odd-page padding.
///
/// Must be emitted after `preamble()` so `brand-primary`/`brand-dark` exist.
pub(crate) fn page_footer(brand: &BrandConfig, timestamps: &str, site: &str) -> String {
    let _ = brand; // colors come from document-scope brand-* variables
    let left = escape_typst(timestamps);
    let right = escape_typst(site);
    format!(
        "#set page(footer: context [\n  \
           #set text(size: _footer-text-size, fill: brand-dark)\n  \
           #line(length: 100%, stroke: _footer-rule + brand-primary)\n  \
           #v(_footer-rule-gap)\n  \
           #grid(columns: (1fr, auto, 1fr), \
             align: (left + horizon, center + horizon, right + horizon),\n    \
             [{left}],\n    \
             [Page #counter(page).display() of #counter(page).final().first()],\n    \
             [{right}],\n  \
           )\n\
         ])\n",
    )
}

/// Generate a `#set page(footer: …)` directive showing only the
/// modified/generated timestamps, centered, with no page number or site label.
///
/// `timestamps` is a pre-formatted string (empty to render an empty footer).
///
/// Must be emitted after `preamble()` so `brand-primary`/`brand-dark` exist.
pub(crate) fn page_footer_timestamps_only(timestamps: &str) -> String {
    let center = escape_typst(timestamps);
    format!(
        "#set page(footer: context [\n  \
           #set text(size: _footer-text-size, fill: brand-dark)\n  \
           #line(length: 100%, stroke: _footer-rule + brand-primary)\n  \
           #v(_footer-rule-gap)\n  \
           #align(center)[{center}]\n\
         ])\n",
    )
}

/// Build the footer timestamp string for the page footer, mirroring the
/// widget's grid footer: `"Modified: Jun 15 4:00 PM | Generated: Jun 15 4:05 PM"`
/// (times shown in the local zone).
///
/// `generated` is shown only when it differs from `modified`. Returns an empty
/// string when neither timestamp parses.
pub(crate) fn footer_timestamps(modified: &str, generated: &str) -> String {
    let mut parts: Vec<String> = vec![];
    if let Some(m) = fmt_stamp(modified) {
        parts.push(format!("Modified: {m}"));
    }
    if generated != modified {
        if let Some(g) = fmt_stamp(generated) {
            parts.push(format!("Generated: {g}"));
        }
    }
    parts.join(" | ")
}

/// Format an RFC 3339 timestamp as `"Jun 15 4:00 PM"` in the local time zone,
/// or `None` if unparseable.
///
/// The stored timestamps are UTC; converting to the system's local zone matches
/// what the widget shows (it renders in the viewer's local zone).
fn fmt_stamp(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let dt = DateTime::parse_from_rfc3339(s).ok()?;
    Some(
        dt.with_timezone(&Local)
            .format("%b %-d %-I:%M %p")
            .to_string(),
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Bare Typst content for a banner label (no surrounding brackets).
fn banner_text(brand: &BrandConfig, escaped: &str) -> String {
    let font_spec = build_font_spec(
        brand.fonts.banner_or_default(),
        brand.fonts.banner_style(),
        Some(brand.fonts.banner_weight_or_default()),
    );
    format!("#text(fill: white, size: _banner-text-size, {font_spec})[#upper[{escaped}]]")
}

/// Banner label wrapped in a grid-cell content block.
fn banner_cell(brand: &BrandConfig, raw: &str) -> String {
    format!("[{}]", banner_text(brand, &escape_typst(raw)))
}

fn build_inner(
    brand: &BrandConfig,
    left: Option<&str>,
    right: Option<&str>,
    logo_path: Option<&str>,
) -> String {
    let logo = logo_path.map(|p| format!("image(\"{p}\", height: _logo-height)"));

    match (left, right, logo.as_deref()) {
        // Both labels + logo → L | logo | R
        (Some(l), Some(r), Some(img)) => format!(
            "#grid(columns: (1fr, auto, 1fr), \
             align: (left + horizon, center + horizon, right + horizon), \
             {}, {img}, {})",
            banner_cell(brand, l),
            banner_cell(brand, r),
        ),
        // Both labels, no logo → L | R
        (Some(l), Some(r), None) => format!(
            "#grid(columns: (1fr, auto), \
             align: (left + horizon, right + horizon), \
             {}, {})",
            banner_cell(brand, l),
            banner_cell(brand, r),
        ),
        // Only left + logo → L | logo
        (Some(l), None, Some(img)) => format!(
            "#grid(columns: (1fr, auto), \
             align: (left + horizon, right + horizon), \
             {}, {img})",
            banner_cell(brand, l),
        ),
        // Only right + logo → logo | R
        (None, Some(r), Some(img)) => format!(
            "#grid(columns: (auto, 1fr), \
             align: (left + horizon, right + horizon), \
             {img}, {})",
            banner_cell(brand, r),
        ),
        // Logo only → centered. `img` is a bare `image(..)` call, so it must be
        // invoked with a leading `#` in this markup context (the grid branches
        // above already sit in code context).
        (None, None, Some(img)) => format!("#align(center)[#{img}]"),
        // Only left, no logo → centered
        (Some(l), None, None) => {
            format!("#align(center)[{}]", banner_text(brand, &escape_typst(l)))
        }
        // Only right, no logo → centered
        (None, Some(r), None) => {
            format!("#align(center)[{}]", banner_text(brand, &escape_typst(r)))
        }
        // Nothing → empty bar
        (None, None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brand::BrandConfig;

    #[test]
    fn test_banner_weight_fallback() {
        let brand = BrandConfig::default();
        // No banner_weight or heading_weight set → defaults to "bold"
        assert_eq!(brand.fonts.banner_weight_or_default(), "bold");
    }

    #[test]
    fn test_banner_font_fallback() {
        let brand = BrandConfig::default();
        // No banner or heading set → "Liberation Sans"
        assert_eq!(brand.fonts.banner_or_default(), "Liberation Sans");
    }

    #[test]
    fn test_page_header_both_no_logo() {
        let brand = BrandConfig::default();
        let out = page_header(&brand, Some("Room A"), Some("Friday"));
        assert!(out.contains("grid"));
        assert!(out.contains("ROOM A") || out.contains("upper"));
        assert!(out.contains("brand-primary"));
    }

    #[test]
    fn test_page_footer_has_page_counter_and_labels() {
        let brand = BrandConfig::default();
        let out = page_footer(&brand, "Modified: Jun 15 4:00 PM", "cosplayamerica.com");
        assert!(out.contains("counter(page)"));
        assert!(out.contains("Modified: Jun 15 4:00 PM"));
        assert!(out.contains("cosplayamerica.com"));
        assert!(out.contains("footer:"));
    }

    #[test]
    fn test_page_header_logo_only_invokes_image() {
        // No labels + a configured logo (the split=none / no header_text case):
        // the logo must be invoked as `#image(...)`, not printed as literal text.
        let mut brand = BrandConfig::default();
        brand.meta.logo_path = Some(std::path::PathBuf::from("logo.svg"));
        let out = page_header(&brand, None, None);
        assert!(out.contains("#image("), "logo must be invoked: {out}");
        assert!(
            !out.contains("[image("),
            "logo must not be bare markup text: {out}"
        );
    }

    #[test]
    fn test_page_header_single_centered() {
        let brand = BrandConfig::default();
        let out = page_header(&brand, None, Some("Friday"));
        assert!(out.contains("align(center)"));
        assert!(!out.contains("grid"));
    }

    #[test]
    fn test_page_header_running_split_no_logo() {
        let brand = BrandConfig::default();
        let out = page_header_running_split(&brand, "[L]", "[R]");
        // No logo configured by default → two-cell grid, no centered logo column.
        assert!(out.contains("grid(columns: (1fr, 1fr)"));
        assert!(out.contains("[L]"));
        assert!(out.contains("[R]"));
        assert!(out.contains("brand-primary"));
    }

    #[test]
    fn test_fmt_stamp_rfc3339() {
        // Formats in the local zone, so build the expectation the same way to
        // stay independent of the machine's time zone.
        let expected = DateTime::parse_from_rfc3339("2026-06-15T16:00:00Z")
            .unwrap()
            .with_timezone(&Local)
            .format("%b %-d %-I:%M %p")
            .to_string();
        assert_eq!(
            fmt_stamp("2026-06-15T16:00:00Z").as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(fmt_stamp("").as_deref(), None);
        assert_eq!(fmt_stamp("not-a-date").as_deref(), None);
    }

    #[test]
    fn test_footer_timestamps_dedups_generated() {
        // Generated == modified → only Modified shown.
        let same = footer_timestamps("2026-06-15T16:00:00Z", "2026-06-15T16:00:00Z");
        assert!(same.starts_with("Modified: "));
        assert!(!same.contains("Generated:"));

        // Differing → both shown, joined with " | ".
        let both = footer_timestamps("2026-06-15T16:00:00Z", "2026-06-15T16:05:00Z");
        assert!(both.starts_with("Modified: "));
        assert!(both.contains(" | Generated: "));
    }
}
