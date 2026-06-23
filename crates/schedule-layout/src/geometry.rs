/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Centralized page/banner/footer geometry.
//!
//! Every dimensional constant that used to be hard-coded inline in the generated
//! Typst lives here, both as a documented Rust constant and as a `#let` variable
//! emitted into the document preamble by [`typst_lets`]. The generators reference
//! those `#let` names (`_content-top`, `_page-edge`, `_col-gutter`, …) instead of
//! repeating literals, so the values stay in sync and the `.typ` is self-describing.
//!
//! ## The top-margin relationship
//!
//! The effective top content margin is *not* a free value — it is the page edge
//! plus the banner bar plus the gap between the banner and the body:
//!
//! ```text
//! content_top = page_edge + banner_height + banner_gap
//! ```
//!
//! Only this sum drives page geometry today; the individual pieces are kept
//! separate so they can each become real [`crate::config::LayoutConfig`] options
//! later. `header-ascent` is a separate adjustment that positions the running
//! header within that top margin.

/// Page edge margin — bottom, left, right, and the bare top page edge (inches).
pub const PAGE_EDGE_IN: f64 = 0.125;
/// Banner bar height at the top of the page (inches). The banner is drawn as a
/// fixed-height block of exactly this height, so the reserved top margin and the
/// visible colored bar always agree (they used to drift apart because the bar
/// grew to fit its text). 0.44in matches the bar the old text-driven banner
/// produced for the default 28 pt label, so letter/tabloid output is preserved.
pub const BANNER_HEIGHT_IN: f64 = 0.44;
/// Gap between the banner bar and the content body (inches). 0.06in keeps the
/// effective top margin at the historical 0.625in now that the banner bar is a
/// fixed 0.44in (0.125 edge + 0.44 banner + 0.06 gap), so body layout is
/// unchanged from when the banner reserved 0.375in and overflowed to 0.44.
pub const BANNER_GAP_IN: f64 = 0.06;
/// Running-header vertical adjustment, emitted as `header-ascent` (inches).
pub const HEADER_ASCENT_IN: f64 = 0.125;
/// Effective top content margin = [`PAGE_EDGE_IN`] + [`BANNER_HEIGHT_IN`] +
/// [`BANNER_GAP_IN`].
pub const CONTENT_TOP_IN: f64 = PAGE_EDGE_IN + BANNER_HEIGHT_IN + BANNER_GAP_IN;
/// Bottom margin widened to fit the page footer (inches).
pub const FOOTER_BOTTOM_IN: f64 = 0.5;
/// Footer bottom margin (inches): the gap kept clear below the footer text. The
/// footer block is `_footer-bottom - _footer-descent` tall and bottom-aligned in
/// the margin, so the text never drops into this strip.
pub const FOOTER_DESCENT_IN: f64 = 0.15;
/// Gutter between body text columns (inches).
pub const COLUMN_GUTTER_IN: f64 = 0.2;
/// Logo image height in the banner (inches).
pub const LOGO_HEIGHT_IN: f64 = 0.3;

/// Banner block inner padding — horizontal (points).
pub const BANNER_INSET_X_PT: f64 = 10.0;
/// Banner block inner padding — vertical (points).
pub const BANNER_INSET_Y_PT: f64 = 5.0;
/// Footer rule (horizontal line) thickness (points).
pub const FOOTER_RULE_PT: f64 = 0.5;
/// Minimum vertical spacing below the footer rule before the footer text
/// (points). The text is otherwise centered in the space below the rule.
pub const FOOTER_RULE_GAP_PT: f64 = 2.0;
/// Gap between the body bottom and the footer rule (inches) — keeps the rule in
/// its historical position near the top of the footer area while the text is
/// centered in the space beneath it.
pub const FOOTER_LINE_GAP_IN: f64 = 0.15;
/// Compact footer body-to-rule gap (inches).
pub const COMPACT_FOOTER_LINE_GAP_IN: f64 = 0.06;

// --- Compact (4×6 postcard / quarter-letter) geometry ---
// Small "photo" papers are ~6in tall instead of 11in, so the standard banner
// and footer reserve proportionally twice as much of the page. These compact
// values keep those bars in line with the letter proportions. Selected for
// [`PaperSize::is_compact`] papers by [`typst_lets`].
/// Compact banner bar height (inches). ~4% of a 6in page, matching the letter
/// banner's proportion; contains the default 13 pt compact banner label (see
/// [`crate::fonts::COMPACT_BANNER_TEXT_SIZE_PT`]).
pub const COMPACT_BANNER_HEIGHT_IN: f64 = 0.24;
/// Compact banner-to-body gap (inches).
pub const COMPACT_BANNER_GAP_IN: f64 = 0.0625;
/// Compact running-header vertical adjustment (inches).
pub const COMPACT_HEADER_ASCENT_IN: f64 = 0.0625;
/// Compact reserved footer height (inches).
pub const COMPACT_FOOTER_BOTTOM_IN: f64 = 0.3;
/// Compact footer bottom margin (inches). Kept slightly above the page edge
/// margin ([`PAGE_EDGE_IN`]) so the footer text never sits tighter to the bottom
/// than the left/right margins do to the sides.
pub const COMPACT_FOOTER_DESCENT_IN: f64 = 0.135;
/// Compact banner logo height (inches).
pub const COMPACT_LOGO_HEIGHT_IN: f64 = 0.16;
/// Compact banner inner padding — horizontal (points).
pub const COMPACT_BANNER_INSET_X_PT: f64 = 5.0;
/// Compact banner inner padding — vertical (points).
pub const COMPACT_BANNER_INSET_Y_PT: f64 = 2.0;

// --- Panel list (guest postcard) layout ---
/// Width of the panel-list color accent bar / its column (points).
pub const PL_ACCENT_COL_PT: f64 = 3.0;
/// Column gutter inside the panel-list time/name grid (points).
pub const PL_COL_GUTTER_PT: f64 = 2.0;
/// Vertical gutter between panel-list rows (ems).
pub const PL_ROW_GUTTER_EM: f64 = 0.8;
/// Space above a panel-list day-separator heading (ems).
pub const PL_HEADING_ABOVE_EM: f64 = 0.8;
/// Space below a panel-list day-separator heading (ems).
pub const PL_HEADING_BELOW_EM: f64 = 0.3;

/// Horizontal shift past which a description panel is treated as having moved to
/// a new column (points), triggering a repeated "(continued)" heading.
pub const COLBREAK_THRESHOLD_PT: f64 = 50.0;

/// Emit the `#let` geometry block for the document preamble.
///
/// Defines all dimensional variables the generators reference. `#let` bindings
/// produce no visible output, and `_content-top` is emitted as the *expression*
/// `_page-edge + _banner-height + _banner-gap` so the top-margin relationship is
/// visible in the generated source.
///
/// Must be emitted inside the preamble (before any `#set page` that uses these).
///
/// `banner_compact`/`footer_compact` select the thin chrome dimensions (used for
/// [`PaperSize::is_compact`] papers by default) for the banner and footer
/// independently. `banner_height`/`footer_bottom` optionally override those bar
/// heights with an explicit Typst length (e.g. `"0.5in"`); when set, the matching
/// inset/gap/logo still come from the compact-or-full set.
///
/// The banner logo height is emitted as `calc.min(...)` of the nominal logo and
/// the bar interior, so the logo always fits the (now fixed-height) banner bar.
pub(crate) fn typst_lets(
    banner_compact: bool,
    banner_height: Option<&str>,
    footer_compact: bool,
    footer_bottom: Option<&str>,
) -> String {
    // Banner-side dimensions follow `banner_compact`.
    let (banner_h_def, banner_gap, header_ascent, logo, inset_x, inset_y) = if banner_compact {
        (
            COMPACT_BANNER_HEIGHT_IN,
            COMPACT_BANNER_GAP_IN,
            COMPACT_HEADER_ASCENT_IN,
            COMPACT_LOGO_HEIGHT_IN,
            COMPACT_BANNER_INSET_X_PT,
            COMPACT_BANNER_INSET_Y_PT,
        )
    } else {
        (
            BANNER_HEIGHT_IN,
            BANNER_GAP_IN,
            HEADER_ASCENT_IN,
            LOGO_HEIGHT_IN,
            BANNER_INSET_X_PT,
            BANNER_INSET_Y_PT,
        )
    };
    // Footer-side dimensions follow `footer_compact`.
    let (footer_bottom_def, footer_descent, footer_line_gap) = if footer_compact {
        (
            COMPACT_FOOTER_BOTTOM_IN,
            COMPACT_FOOTER_DESCENT_IN,
            COMPACT_FOOTER_LINE_GAP_IN,
        )
    } else {
        (FOOTER_BOTTOM_IN, FOOTER_DESCENT_IN, FOOTER_LINE_GAP_IN)
    };

    // Explicit length overrides replace the bar height; otherwise emit the
    // selected default as inches.
    let banner_h = banner_height
        .map(str::to_string)
        .unwrap_or_else(|| format!("{banner_h_def}in"));
    let footer_b = footer_bottom
        .map(str::to_string)
        .unwrap_or_else(|| format!("{footer_bottom_def}in"));

    format!(
        "#let _page-edge = {page_edge}in\n\
         #let _banner-height = {banner_h}\n\
         #let _banner-gap = {banner_gap}in\n\
         #let _header-ascent = {ascent}in\n\
         #let _content-top = _page-edge + _banner-height + _banner-gap\n\
         #let _footer-bottom = {footer_b}\n\
         #let _footer-descent = {footer_descent}in\n\
         #let _footer-line-gap = {footer_line_gap}in\n\
         #let _col-gutter = {gutter}in\n\
         #let _banner-inset = (x: {inset_x}pt, y: {inset_y}pt)\n\
         #let _logo-height = calc.min({logo}in, _banner-height - 2 * _banner-inset.y)\n\
         #let _footer-rule = {footer_rule}pt\n\
         #let _footer-rule-gap = {footer_gap}pt\n\
         #let _pl-accent-col = {pl_accent}pt\n\
         #let _pl-col-gutter = {pl_col_gutter}pt\n\
         #let _pl-row-gutter = {pl_row}em\n\
         #let _pl-heading-above = {pl_h_above}em\n\
         #let _pl-heading-below = {pl_h_below}em\n\
         #let _colbreak-threshold = {colbreak}\n",
        page_edge = PAGE_EDGE_IN,
        banner_h = banner_h,
        banner_gap = banner_gap,
        ascent = header_ascent,
        footer_b = footer_b,
        footer_descent = footer_descent,
        footer_line_gap = footer_line_gap,
        gutter = COLUMN_GUTTER_IN,
        logo = logo,
        inset_x = inset_x,
        inset_y = inset_y,
        footer_rule = FOOTER_RULE_PT,
        footer_gap = FOOTER_RULE_GAP_PT,
        pl_accent = PL_ACCENT_COL_PT,
        pl_col_gutter = PL_COL_GUTTER_PT,
        pl_row = PL_ROW_GUTTER_EM,
        pl_h_above = PL_HEADING_ABOVE_EM,
        pl_h_below = PL_HEADING_BELOW_EM,
        colbreak = COLBREAK_THRESHOLD_PT,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_top_relationship() {
        // The effective top margin is page edge + banner + gap.
        assert!((CONTENT_TOP_IN - (PAGE_EDGE_IN + BANNER_HEIGHT_IN + BANNER_GAP_IN)).abs() < 1e-9);
    }

    #[test]
    fn test_typst_lets_defines_expected_vars() {
        let lets = typst_lets(false, None, false, None);
        assert!(lets.contains("#let _page-edge = 0.125in"));
        assert!(lets.contains("#let _content-top = _page-edge + _banner-height + _banner-gap"));
        assert!(lets.contains("#let _col-gutter = 0.2in"));
        assert!(lets.contains("#let _banner-inset = (x: 10pt, y: 5pt)"));
        assert!(lets.contains("#let _footer-rule = 0.5pt"));
        // The logo height is capped to the bar interior so it cannot overflow.
        assert!(lets.contains("#let _logo-height = calc.min("));
    }

    #[test]
    fn test_typst_lets_compact_geometry() {
        // Compact banner + footer use the thinner sets.
        let lets = typst_lets(true, None, true, None);
        assert!(lets.contains(&format!("#let _banner-height = {COMPACT_BANNER_HEIGHT_IN}in")));
        assert!(lets.contains(&format!("#let _footer-bottom = {COMPACT_FOOTER_BOTTOM_IN}in")));
        assert!(lets.contains("#let _banner-inset = (x: 5pt, y: 2pt)"));
        // Full-size keeps the standard banner.
        let std = typst_lets(false, None, false, None);
        assert!(std.contains(&format!("#let _banner-height = {BANNER_HEIGHT_IN}in")));
    }

    #[test]
    fn test_typst_lets_independent_banner_footer() {
        // Banner and footer compactness are independent (e.g. compact footer with
        // a full banner).
        let lets = typst_lets(false, None, true, None);
        assert!(lets.contains(&format!("#let _banner-height = {BANNER_HEIGHT_IN}in")));
        assert!(lets.contains(&format!("#let _footer-bottom = {COMPACT_FOOTER_BOTTOM_IN}in")));
    }

    #[test]
    fn test_typst_lets_explicit_height_override() {
        let lets = typst_lets(false, Some("0.5in"), false, Some("0.4in"));
        assert!(lets.contains("#let _banner-height = 0.5in"));
        assert!(lets.contains("#let _footer-bottom = 0.4in"));
    }
}
