/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Branded page-header bar used by all layout formats.

use chrono::{DateTime, Local};

use crate::brand::BrandConfig;
use crate::typst_gen::escape_typst;

/// Generate a `#set page(header: …)` Typst directive for all layout formats.
///
/// `logo_path` is the already-resolved absolute path to the logo image, or
/// `None` to suppress the logo.  Pass the result of
/// [`crate::document::resolve_logo`] here.
///
/// Logo placement follows these rules:
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
pub(crate) fn page_header(
    brand: &BrandConfig,
    logo_path: Option<&str>,
    left: Option<&str>,
    right: Option<&str>,
) -> String {
    let _ = brand; // styling uses document-scope brand-* variables
    let inner = build_inner(left, right, logo_path);
    format!("#set page(header: {})\n", banner_block(&inner))
}

/// Wrap banner `inner` content in the fixed-height colored bar.
///
/// The block is exactly `_banner-height` tall (so the visible bar matches the
/// reserved top margin) with its content vertically centered. Inner grids
/// already align their cells `+ horizon`; the surrounding `align(horizon)` keeps
/// single-label / logo-only content centered too.
fn banner_block(inner: &str) -> String {
    format!(
        "block(fill: brand-primary, width: 100%, height: _banner-height, \
         inset: _banner-inset)[\n  #align(horizon)[{inner}]\n]",
    )
}

/// Generate a `#set page(header: …)` directive whose right-hand label is *raw
/// Typst content* (e.g. a `context` expression for a running header) rather
/// than a literal string.
///
/// `logo_path` is the already-resolved absolute path to the logo image, or
/// `None` to suppress it.  When a logo is given it sits on the left and the
/// content is right-aligned; without a logo the content is centered.  Styling
/// matches [`page_header`] (ALL CAPS, banner font, 28 pt).  `right_content` is
/// inserted verbatim, so the caller is responsible for it being valid Typst.
pub(crate) fn page_header_running(
    brand: &BrandConfig,
    logo_path: Option<&str>,
    right_content: &str,
) -> String {
    let _ = brand;
    // Auto-fit: measure the upper-cased label at the nominal banner size and, if
    // it is wider than the space the layout gives it (the 1fr cell beside the
    // logo, or the full bar when centered), shrink the font just enough to keep
    // it on one line. `right_content` resolves the running per-page label, so the
    // sizing happens per page inside the `layout`/`measure` context.
    let label = fit_banner_label(right_content);

    let inner = match logo_path {
        Some(p) => format!(
            "#grid(columns: (auto, 1fr), align: (left + horizon, right + horizon), \
             image(\"{p}\", height: _logo-height), [{label}])",
        ),
        None => format!("#align(center)[{label}]"),
    };

    format!("#set page(header: {})\n", banner_block(&inner))
}

/// Generate a `#set page(header: …)` directive with *two* raw-Typst content
/// slots — a left label and a right label — for running headers that vary per
/// page (e.g. room signs, where each page shows its room on the left and day on
/// the right).
///
/// `logo_path` is the already-resolved absolute path to the logo image, or
/// `None` to suppress it.  With a logo the slots sit either side of a centered
/// logo (L | logo | R); without one, they split the bar evenly (L | R).  Both
/// contents are styled like the other banners (ALL CAPS, banner font, 28 pt)
/// and inserted verbatim.
pub(crate) fn page_header_running_split(
    brand: &BrandConfig,
    logo_path: Option<&str>,
    left_content: &str,
    right_content: &str,
) -> String {
    let _ = brand;
    // Auto-fit both slots: a long entity label (e.g. a guest's name) would
    // otherwise overrun its 1fr cell at the nominal banner size and collide with
    // the centered logo. `fit_banner_label` shrinks each label to stay on one
    // line within the width its cell offers (resolved per page via `layout`).
    let left = fit_banner_label(left_content);
    let right = fit_banner_label(right_content);

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

    format!("#set page(header: {})\n", banner_block(&inner))
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
    footer_context(&format!(
        "#grid(columns: (1fr, auto, 1fr), \
           align: (left + horizon, center + horizon, right + horizon),\n    \
           [{left}],\n    \
           [Page #counter(page).display() of #counter(page).final().first()],\n    \
           [{right}],\n  \
         )",
    ))
}

/// Wrap footer `content` (a grid or `align` expression) in the centered footer
/// block. The horizontal rule sits `_footer-line-gap` below the body bottom — its
/// historical position — and `content` is vertically centered in the space
/// beneath the rule (`v(1fr)` either side), with a minimum `_footer-rule-gap`
/// below the rule. The block's bottom rests at `footer-descent` from the page
/// edge, so the text never drops into the bottom margin.
fn footer_context(content: &str) -> String {
    // The outer block fills the bottom margin from the body down to the bottom
    // page margin: `footer-descent` (set in the preamble) pins its bottom at
    // `_footer-descent` above the page edge, and its height ends at the body
    // bottom. The rule sits `_footer-line-gap` below the body; the inner block
    // takes the remaining height (`1fr`) and vertically centers the text with a
    // minimum `_footer-rule-gap` below the rule. (A trailing `v(1fr)` would be
    // collapsed at the block end, so centering uses an explicit sized block.)
    format!(
        "#set page(footer: context [\n  \
           #set text(size: _footer-text-size, fill: brand-dark)\n  \
           #block(width: 100%, height: _footer-bottom - _footer-descent)[\n    \
             #v(_footer-line-gap)\n    \
             #line(length: 100%, stroke: _footer-rule + brand-primary)\n    \
             #block(width: 100%, height: 1fr, inset: (top: _footer-rule-gap))[\n      \
               #align(horizon)[{content}]\n    \
             ]\n  \
           ]\n\
         ])\n",
    )
}

/// Generate a `#set page(footer: …)` directive like [`page_footer`] but with the
/// centered global "Page X of N" replaced by a *per-section* counter labelled by
/// the running section — e.g. `"Avera: Page 1 of 4"`.
///
/// The center cell is a `context` expression that reads the invisible
/// `<section>` markers (see `document::section_marker`). It groups *contiguous*
/// sections that share the same entity label — the marker's `left` field, or
/// `right` when `left` is empty — so a presenter split by day counts across all
/// of that guest's days (e.g. Thursday→Sunday show "Avera: Page 1/2/3/4 of 4")
/// rather than resetting every day. It prints the page-within-group out of the
/// group's page count, derived from the surrounding markers (and the document's
/// final page).
///
/// On a page with no preceding marker (no split active) it degrades to the same
/// global "Page X of N" that [`page_footer`] shows.
///
/// `timestamps` (left) and `site` (right) occupy the same slots as
/// [`page_footer`]. Must be emitted after `preamble()` so `brand-*` exist.
pub(crate) fn page_footer_section_pages(timestamps: &str, site: &str) -> String {
    let left = escape_typst(timestamps);
    let right = escape_typst(site);
    footer_context(&format!(
        "#grid(columns: (1fr, auto, 1fr), \
           align: (left + horizon, center + horizon, right + horizon),\n    \
           [{left}],\n    \
           {{\n      \
               let _pg = here().page()\n      \
               let _ms = query(<section>)\n      \
               let _final = counter(page).final().first()\n      \
               let _key = m => if m.value.left != \"\" {{ m.value.left }} else {{ m.value.right }}\n      \
               let _before = _ms.filter(m => m.location().page() <= _pg)\n      \
               if _before.len() == 0 {{\n        \
                 [Page #_pg of #_final]\n      \
               }} else {{\n        \
                 let _idx = _before.len() - 1\n        \
                 let _k = _key(_ms.at(_idx))\n        \
                 let _start_i = _idx\n        \
                 while _start_i > 0 and _key(_ms.at(_start_i - 1)) == _k {{ _start_i -= 1 }}\n        \
                 let _end_i = _idx\n        \
                 while _end_i + 1 < _ms.len() and _key(_ms.at(_end_i + 1)) == _k {{ _end_i += 1 }}\n        \
                 let _start = _ms.at(_start_i).location().page()\n        \
                 let _end = if _end_i + 1 < _ms.len() {{ _ms.at(_end_i + 1).location().page() - 1 }} else {{ _final }}\n        \
                 [#_k: Page #(_pg - _start + 1) of #(_end - _start + 1)]\n      \
               }}\n    \
             }},\n    \
             [{right}],\n  \
         )",
    ))
}

/// Generate a `#set page(footer: …)` directive showing only the
/// modified/generated timestamps, centered, with no page number or site label.
///
/// `timestamps` is a pre-formatted string (empty to render an empty footer).
///
/// Must be emitted after `preamble()` so `brand-primary`/`brand-dark` exist.
pub(crate) fn page_footer_timestamps_only(timestamps: &str) -> String {
    let center = escape_typst(timestamps);
    footer_context(&format!("#align(center)[{center}]"))
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
///
/// The banner typeface is the global `_banner-font` dict from the preamble
/// (`fonts::typst_lets`), spread into the text call.
fn banner_text(escaped: &str) -> String {
    format!("#text(fill: white, size: _banner-text-size, .._banner-font)[#upper[{escaped}]]")
}

/// Banner label wrapped in a grid-cell content block.
fn banner_cell(raw: &str) -> String {
    format!("[{}]", banner_text(&escape_typst(raw)))
}

/// Build an auto-shrinking banner label from raw Typst content.
///
/// `content` is inserted verbatim (a literal string or a `#context` expression
/// that resolves a running per-page label). The returned Typst measures the
/// upper-cased label at `_banner-text-size` and, if it is wider than the width
/// the surrounding layout offers, scales the font down by exactly the overflow
/// ratio so the label stays on a single line. Labels that already fit keep the
/// nominal size.
fn fit_banner_label(content: &str) -> String {
    format!(
        "#layout(_sz => {{\n    \
           let _b = upper[#text(.._banner-font)[{content}]]\n    \
           let _m = measure(text(size: _banner-text-size)[#_b])\n    \
           let _s = if _m.width > _sz.width and _m.width > 0pt {{ \
             _banner-text-size * (_sz.width / _m.width) \
           }} else {{ _banner-text-size }}\n    \
           text(fill: white, size: _s)[#_b]\n  \
         }})",
    )
}

fn build_inner(left: Option<&str>, right: Option<&str>, logo_path: Option<&str>) -> String {
    let logo = logo_path.map(|p| format!("image(\"{p}\", height: _logo-height)"));

    match (left, right, logo.as_deref()) {
        // Both labels + logo → L | logo | R
        (Some(l), Some(r), Some(img)) => format!(
            "#grid(columns: (1fr, auto, 1fr), \
             align: (left + horizon, center + horizon, right + horizon), \
             {}, {img}, {})",
            banner_cell(l),
            banner_cell(r),
        ),
        // Both labels, no logo → L | R
        (Some(l), Some(r), None) => format!(
            "#grid(columns: (1fr, auto), \
             align: (left + horizon, right + horizon), \
             {}, {})",
            banner_cell(l),
            banner_cell(r),
        ),
        // Only left + logo → L | logo
        (Some(l), None, Some(img)) => format!(
            "#grid(columns: (1fr, auto), \
             align: (left + horizon, right + horizon), \
             {}, {img})",
            banner_cell(l),
        ),
        // Only right + logo → logo | R
        (None, Some(r), Some(img)) => format!(
            "#grid(columns: (auto, 1fr), \
             align: (left + horizon, right + horizon), \
             {img}, {})",
            banner_cell(r),
        ),
        // Logo only → centered. `img` is a bare `image(..)` call, so it must be
        // invoked with a leading `#` in this markup context (the grid branches
        // above already sit in code context).
        (None, None, Some(img)) => format!("#align(center)[#{img}]"),
        // Only left, no logo → centered
        (Some(l), None, None) => {
            format!("#align(center)[{}]", banner_text(&escape_typst(l)))
        }
        // Only right, no logo → centered
        (None, Some(r), None) => {
            format!("#align(center)[{}]", banner_text(&escape_typst(r)))
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
        let out = page_header(&brand, None, Some("Room A"), Some("Friday"));
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
        // No labels + a logo path supplied: the logo must be invoked as `#image(...)`,
        // not printed as literal text.
        let brand = BrandConfig::default();
        let out = page_header(&brand, Some("logo.svg"), None, None);
        assert!(out.contains("#image("), "logo must be invoked: {out}");
        assert!(
            !out.contains("[image("),
            "logo must not be bare markup text: {out}"
        );
    }

    #[test]
    fn test_page_header_single_centered() {
        let brand = BrandConfig::default();
        let out = page_header(&brand, None, None, Some("Friday"));
        assert!(out.contains("align(center)"));
        assert!(!out.contains("grid"));
    }

    #[test]
    fn test_page_header_running_split_no_logo() {
        let brand = BrandConfig::default();
        let out = page_header_running_split(&brand, None, "[L]", "[R]");
        // No logo → two-cell grid, no centered logo column.
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
