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
/// Banner bar height reserved at the top of the page (inches).
pub const BANNER_HEIGHT_IN: f64 = 0.375;
/// Gap between the banner bar and the content body (inches).
pub const BANNER_GAP_IN: f64 = 0.125;
/// Running-header vertical adjustment, emitted as `header-ascent` (inches).
pub const HEADER_ASCENT_IN: f64 = 0.125;
/// Effective top content margin = [`PAGE_EDGE_IN`] + [`BANNER_HEIGHT_IN`] +
/// [`BANNER_GAP_IN`] (== 0.625in today).
pub const CONTENT_TOP_IN: f64 = PAGE_EDGE_IN + BANNER_HEIGHT_IN + BANNER_GAP_IN;
/// Bottom margin widened to fit the page footer (inches).
pub const FOOTER_BOTTOM_IN: f64 = 0.5;
/// Distance from the page bottom edge to the footer baseline (inches).
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
/// Vertical spacing below the footer rule (points).
pub const FOOTER_RULE_GAP_PT: f64 = 2.0;

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
pub(crate) fn typst_lets() -> String {
    format!(
        "#let _page-edge = {page_edge}in\n\
         #let _banner-height = {banner_h}in\n\
         #let _banner-gap = {banner_gap}in\n\
         #let _header-ascent = {ascent}in\n\
         #let _content-top = _page-edge + _banner-height + _banner-gap\n\
         #let _footer-bottom = {footer_bottom}in\n\
         #let _footer-descent = {footer_descent}in\n\
         #let _col-gutter = {gutter}in\n\
         #let _logo-height = {logo}in\n\
         #let _banner-inset = (x: {inset_x}pt, y: {inset_y}pt)\n\
         #let _footer-rule = {footer_rule}pt\n\
         #let _footer-rule-gap = {footer_gap}pt\n\
         #let _pl-accent-col = {pl_accent}pt\n\
         #let _pl-col-gutter = {pl_col_gutter}pt\n\
         #let _pl-row-gutter = {pl_row}em\n\
         #let _pl-heading-above = {pl_h_above}em\n\
         #let _pl-heading-below = {pl_h_below}em\n\
         #let _colbreak-threshold = {colbreak}\n",
        page_edge = PAGE_EDGE_IN,
        banner_h = BANNER_HEIGHT_IN,
        banner_gap = BANNER_GAP_IN,
        ascent = HEADER_ASCENT_IN,
        footer_bottom = FOOTER_BOTTOM_IN,
        footer_descent = FOOTER_DESCENT_IN,
        gutter = COLUMN_GUTTER_IN,
        logo = LOGO_HEIGHT_IN,
        inset_x = BANNER_INSET_X_PT,
        inset_y = BANNER_INSET_Y_PT,
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
        // The effective top margin must remain 0.625in (page edge + banner + gap).
        assert!((CONTENT_TOP_IN - 0.625).abs() < 1e-9);
    }

    #[test]
    fn test_typst_lets_defines_expected_vars() {
        let lets = typst_lets();
        assert!(lets.contains("#let _page-edge = 0.125in"));
        assert!(lets.contains("#let _content-top = _page-edge + _banner-height + _banner-gap"));
        assert!(lets.contains("#let _col-gutter = 0.2in"));
        assert!(lets.contains("#let _banner-inset = (x: 10pt, y: 5pt)"));
        assert!(lets.contains("#let _footer-rule = 0.5pt"));
    }
}
