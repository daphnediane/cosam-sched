/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Layout configuration: paper, orientation, content/split modes, and the
//! per-job [`LayoutConfig`] that drives the document builder.

use crate::color::ColorMode;

/// Page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Landscape,
    Portrait,
}

impl Orientation {
    /// Returns `true` for landscape orientation.
    pub fn is_landscape(self) -> bool {
        matches!(self, Orientation::Landscape)
    }
}

/// Paper size for output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaperSize {
    Letter,
    Legal,
    #[default]
    Tabloid,
    SuperB,
    /// Custom 30"×20" poster. Supports both portrait and landscape orientations.
    Poster,
    Postcard4x6,
}

impl PaperSize {
    /// Returns `(width_mm, height_mm)` in portrait orientation.
    pub fn dimensions_mm(&self) -> (f64, f64) {
        match self {
            PaperSize::Letter => (215.9, 279.4),
            PaperSize::Legal => (215.9, 355.6),
            PaperSize::Tabloid => (279.4, 431.8),
            PaperSize::SuperB => (330.2, 482.6),
            PaperSize::Poster => (508.0, 762.0), // 20"×30" portrait basis
            PaperSize::Postcard4x6 => (101.6, 152.4),
        }
    }

    /// Typst paper name used in `#set page(paper: ...)`.
    /// Returns `None` for sizes that require explicit `width`/`height` dimensions
    /// (e.g. `Poster`).
    pub fn typst_name(&self) -> Option<&'static str> {
        match self {
            PaperSize::Letter => Some("us-letter"),
            PaperSize::Legal => Some("us-legal"),
            PaperSize::Tabloid => Some("us-tabloid"),
            PaperSize::SuperB => Some("iso-b3"),
            PaperSize::Poster => None,
            PaperSize::Postcard4x6 => Some("a6"),
        }
    }

    /// Subdirectory name used under the layout output root.
    pub fn dir_name(&self) -> &'static str {
        match self {
            PaperSize::Letter => "letter",
            PaperSize::Legal => "legal",
            PaperSize::Tabloid => "tabloid",
            PaperSize::SuperB => "super-b",
            PaperSize::Poster => "poster",
            PaperSize::Postcard4x6 => "postcard",
        }
    }

    /// Number of columns for a description/workshops listing on this paper.
    ///
    /// Column counts match the legacy `schedule-to-html` CSS files, targeting
    /// a fixed ~3-inch column width across paper sizes.
    pub fn description_columns(&self, orientation: Orientation) -> u32 {
        match self {
            PaperSize::Letter => {
                if orientation.is_landscape() {
                    4
                } else {
                    3
                }
            }
            PaperSize::Legal => {
                if orientation.is_landscape() {
                    4
                } else {
                    3
                }
            }
            PaperSize::Tabloid | PaperSize::SuperB => {
                if orientation.is_landscape() {
                    5
                } else {
                    4
                }
            }
            PaperSize::Poster => 5,
            PaperSize::Postcard4x6 => 1,
        }
    }

    /// Number of columns for a flyer-schedule page on this paper.
    ///
    /// The flyer format devotes the left half of the first page (rounded up) to
    /// the day grid and flows descriptions through the remaining columns, so the
    /// total must be even-friendly: letter uses 4 columns, legal and larger use
    /// 6.  Portrait falls back to narrower counts.
    pub fn flyer_columns(&self, orientation: Orientation) -> u32 {
        match self {
            PaperSize::Letter => {
                if orientation.is_landscape() {
                    4
                } else {
                    2
                }
            }
            PaperSize::Legal | PaperSize::Tabloid | PaperSize::SuperB | PaperSize::Poster => {
                if orientation.is_landscape() {
                    6
                } else {
                    4
                }
            }
            PaperSize::Postcard4x6 => 2,
        }
    }

    /// Base font size (as a Typst length string) for body text on this paper.
    ///
    /// The `Poster` size uses a larger base font so that panels are legible at
    /// reading distance from a printed 30"×20" sheet.
    pub fn base_font_pt(&self) -> &'static str {
        match self {
            PaperSize::Poster => "10pt",
            _ => "9pt",
        }
    }
}

/// How to split sections by entity (room or presenter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionSplit {
    /// One section per room.
    Room,
    /// One section per presenter.
    Presenter,
}

/// A single time slot within a custom timeline.
/// Slots run from their `time` until either `end_time` (if set) or the next slot's time.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomTimeSlot {
    /// Display label for this slot (e.g., "Slot A", "Friday Morning Workshop").
    pub label: String,
    /// Start time in ISO 8601 format (e.g., "2026-06-26T12:00").
    pub time: String,
    /// Optional explicit end time. Panels at or after this time are excluded from this slot.
    /// When absent, the slot runs until the next slot's start time (which may be on a later
    /// day). The last slot in the timeline has no upper bound.
    pub end_time: Option<String>,
    /// Override for base font size (e.g., "14pt") for this slot's section.
    /// If None, uses the job's global `base_font_pt` setting.
    pub base_font_pt: Option<String>,
    /// Override for grid font size (e.g., "10pt") for this slot's section.
    /// If None, uses the job's global `grid_font_pt` setting (or base_font_pt).
    pub grid_font_pt: Option<String>,
}

/// A named custom timeline consisting of time slots.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CustomTimeline {
    /// Ordered list of time slots in chronological order.
    /// Each slot runs from its `time` until the next slot's time.
    /// Panels before the first slot get a "Before <label>" section unless the first
    /// slot has an explicit `end_time` (windowed), in which case they are excluded.
    pub slots: Vec<CustomTimeSlot>,
}

/// How to split sections by time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeSplit {
    /// One section per calendar day.
    Day,
    /// One section per AM/PM half-day (geometric noon boundary).
    HalfDay,
    /// One section per timeline entry (data-driven: splits on the schedule's
    /// SPLIT/timeline panels, e.g. "Friday Morning", "Friday Afternoon").
    Timeline,
    /// One section per custom time slot. The timeline is fully resolved and
    /// embedded — `schedule-layout` does not need to know about other timelines
    /// or the config file structure.
    Custom(CustomTimeline),
}

/// Output file format for a layout job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutFormat {
    /// Typst source compiled to PDF (the default pipeline).
    #[default]
    Typst,
    /// Adobe InDesign Markup Language package (`.idml`). Feature-gated behind the
    /// `idml` crate feature; see [`crate::idml`].
    Idml,
}

/// Page-footer content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FooterMode {
    /// Modified/generated timestamps, a centered page number, and the site/org
    /// label (the default).
    #[default]
    Full,
    /// Modified/generated timestamps only — no page number or site label.
    TimestampOnly,
    /// Per-section pagination: the timestamps and site keep their slots, but the
    /// centered global "Page X of N" is replaced with a per-section counter
    /// labelled by the running section — e.g. `"Avera: Page 1 of 4"`. Requires an
    /// active split (section markers); falls back to the global counter on pages
    /// with no marker.
    SectionPages,
    /// No footer at all.
    None,
}

/// Which content a section renders, with how to split it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentMode {
    /// Schedule grid and panel descriptions side by side.
    Both {
        section: Option<SectionSplit>,
        time: TimeSplit,
    },
    /// Schedule grid only — no descriptions.
    GridOnly {
        section: Option<SectionSplit>,
        time: TimeSplit,
    },
    /// Panel descriptions only — no grid.
    DescriptionOnly {
        section: Option<SectionSplit>,
        time: Option<TimeSplit>,
    },
    /// Compact panel list (name + time + room), the former guest-postcard layout.
    PanelList {
        section: Option<SectionSplit>,
        time: Option<TimeSplit>,
    },
}

impl Default for ContentMode {
    fn default() -> Self {
        ContentMode::Both {
            section: None,
            time: TimeSplit::Day,
        }
    }
}

impl ContentMode {
    /// The section (entity) split, if any.
    pub fn section_split(&self) -> Option<SectionSplit> {
        match *self {
            ContentMode::Both { section, .. }
            | ContentMode::GridOnly { section, .. }
            | ContentMode::DescriptionOnly { section, .. }
            | ContentMode::PanelList { section, .. } => section,
        }
    }

    /// The time split, if any.
    pub fn time_split(&self) -> Option<TimeSplit> {
        match self {
            ContentMode::Both { time, .. } | ContentMode::GridOnly { time, .. } => {
                Some(time.clone())
            }
            ContentMode::DescriptionOnly { time, .. } | ContentMode::PanelList { time, .. } => {
                time.clone()
            }
        }
    }

    /// Whether any split is active (section or time).
    pub fn has_split(&self) -> bool {
        self.section_split().is_some() || self.time_split().is_some()
    }

    /// Whether both a section split and a time split are active (two-slot running header).
    pub fn is_two_dim(&self) -> bool {
        self.section_split().is_some() && self.time_split().is_some()
    }

    /// Whether this content draws the schedule grid.
    pub fn shows_grid(&self) -> bool {
        matches!(
            *self,
            ContentMode::Both { .. } | ContentMode::GridOnly { .. }
        )
    }

    /// Whether this content draws panel text (descriptions or list).
    pub fn shows_text(&self) -> bool {
        matches!(
            *self,
            ContentMode::Both { .. }
                | ContentMode::DescriptionOnly { .. }
                | ContentMode::PanelList { .. }
        )
    }
}

/// Which panels to include in the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelFilter {
    /// Every scheduled panel (the default).
    #[default]
    All,
    /// Workshop panels only (premium workshops included; cafe excluded).
    Workshops,
    /// Premium workshops only.
    Premium,
}

/// Complete configuration for a single layout job.
#[derive(Debug, Clone, Default)]
pub struct LayoutConfig {
    pub paper: PaperSize,
    /// Output file format. Defaults to [`LayoutFormat::Typst`], so existing
    /// Typst/PDF behavior is unchanged unless a job opts into IDML.
    pub format: LayoutFormat,
    /// What to render and how to split it.
    pub content: ContentMode,
    /// Which panels to include.
    pub panel_filter: PanelFilter,
    pub orientation: Orientation,
    /// Include private panels and unlisted (uncredited) presenters in this job.
    ///
    /// This selects the *private* schedule view: the data must have been built
    /// with [`ScheduleData::from_schedule`]`(.., private = true)`. The renderer
    /// draws whatever data it is handed, so callers pair this flag with the
    /// matching dataset. `false` (the default) is the public view.
    ///
    /// [`ScheduleData::from_schedule`]: crate::model::ScheduleData::from_schedule
    pub include_private: bool,
    /// Color or black-and-white output.
    pub color_mode: ColorMode,
    /// Override for the number of layout columns. If `None`, each content mode
    /// uses its own per-paper default (e.g. [`PaperSize::flyer_columns`]).
    pub columns: Option<u32>,
    /// Page-footer content.
    pub footer: FooterMode,
    /// For per-presenter day grids, whether to show *only* the days a presenter
    /// is scheduled. `None`/`Some(true)` (the default) skips days the presenter
    /// has no panels. `Some(false)` instead still emits those days — the full day
    /// grid (every panel that day) with nothing highlighted — so every guest's
    /// booklet shows the same set of days. Honored for the presenter × time split
    /// (the guest-schedule case).
    pub matching_only: Option<bool>,
    /// Insert a blank page so each section starts on an odd page (double-sided
    /// booklet printing).
    pub double_sided: bool,
    /// Optional header text: shown on the left for 1-D splits, on the right when
    /// there is no split, and omitted for 2-D splits (where both header slots
    /// carry the running entity/day labels).
    pub header_text: Option<String>,
    /// Override for base font size (e.g., "14pt"). If None, uses paper's default.
    pub base_font_pt: Option<String>,
    /// Override for grid event text size (e.g., "8pt"). If None, uses base_font_pt.
    pub grid_font_pt: Option<String>,
    /// Page background color (hex, `luma(...)`, or a named Typst color). `None` =
    /// the default white page.
    pub page_fill: Option<String>,
    /// Fill for empty (no-event) grid cells. Set this to keep empties from
    /// blending into a tinted [`page_fill`]. `None` = the built-in light gray.
    pub empty_grid_fill: Option<String>,
    /// Fade panels that conflict with the highlighted selection in presenter
    /// schedules: a non-highlighted event overlapping one of the guest's own
    /// panels is dimmed, the "you're busy elsewhere" cue from schedule-to-html.
    /// `false` (the default) leaves conflicting panels at full strength.
    pub dim_conflict: bool,
    /// Render description panels as bordered cards (colored left spine + border)
    /// instead of the original full-height left accent bar.
    pub cards: bool,
    /// Card background color when [`cards`](Self::cards) is set. `None` = white.
    pub card_fill: Option<String>,
    /// Override the gutter between body-text columns (e.g. `"0.25in"`). `None` =
    /// the default `_col-gutter` (0.2in).
    pub column_gap: Option<String>,
    /// Gap between cards (e.g. `"10pt"`); applies when [`cards`](Self::cards) is
    /// set. The literal `"column"` (also `"col"`/`"gutter"`) means "match the
    /// column gutter". `None` also matches the column gutter.
    pub card_gap: Option<String>,
    /// Logo to show in the page header. `None` suppresses the logo entirely.
    /// `Some("brand")` (the default) resolves the `"brand"` alias from
    /// `[logos]` in `brand.toml`. Any other string is looked up as a named
    /// alias first, then as a bare filename within `logo_dir`.
    pub logo: Option<String>,
    /// Override the banner text size (e.g. `"18pt"`). `None` uses the
    /// built-in default ([`crate::fonts::BANNER_TEXT_SIZE_PT`] = 28 pt).
    /// Useful for postcards or jobs with long presenter names.
    pub banner_text_pt: Option<String>,
    /// Per-job micro font family, overriding the brand's
    /// [`micro`](crate::brand::BrandFonts::micro). `Some("none")` disables the
    /// micro substitution for this job even when the brand sets one; `None`
    /// inherits the brand value.
    pub micro: Option<String>,
    /// Per-job micro font style, overriding the brand's `micro_style`.
    pub micro_style: Option<String>,
    /// Per-job micro font weight, overriding the brand's `micro_weight`.
    pub micro_weight: Option<String>,
    /// Per-job micro size threshold in points: text below this switches to the
    /// micro font. Overrides the brand's `micro_max_pt`; `None` inherits it
    /// (then the built-in [`crate::fonts::MICRO_MAX_PT`] default).
    pub micro_max_pt: Option<f64>,
    /// Whether a full-page schedule grid is fit onto a single page: the grid is
    /// compressed (and text-heavy cells condensed to fit) only when it would
    /// overflow. `None` uses the per-content default — on for [`ContentMode::GridOnly`],
    /// off otherwise. Set `Some(false)` to let a grid flow naturally (and
    /// paginate) instead, or `Some(true)` to force fitting.
    pub fit_grid: Option<bool>,
    /// Optional URL encoded as a QR code placed in the bottom-right corner of
    /// every page. `None` omits the QR entirely. The URL is also shown in small
    /// text below the code.
    pub qr_url: Option<String>,
    /// Optional caption shown above the QR code (e.g. `"Register Here"`). Only
    /// used when [`qr_url`](Self::qr_url) is set.
    pub qr_msg: Option<String>,
    /// Caption text size as a Typst length (e.g. `"12.5pt"`). `None` uses
    /// [`crate::qr::DEFAULT_CAPTION_SIZE`].
    pub qr_caption_pt: Option<String>,
    /// URL text size as a Typst length (e.g. `"10pt"`). `None` uses
    /// [`crate::qr::DEFAULT_URL_SIZE`].
    pub qr_url_pt: Option<String>,
    /// QR code size as a Typst length (e.g. `"0.75in"`). `None` uses the default
    /// ([`crate::qr::DEFAULT_QR_SIZE`]). Only used when [`qr_url`](Self::qr_url)
    /// is set.
    pub qr_size: Option<String>,
}

impl LayoutConfig {
    /// Resolve the column count, honoring the [`columns`](Self::columns)
    /// override and falling back to `default` (clamped to at least 1).
    pub fn effective_columns(&self, default: u32) -> u32 {
        self.columns.unwrap_or(default).max(1)
    }

    /// Get the effective base font size for this layout.
    pub fn effective_font_pt(&self) -> &str {
        self.base_font_pt
            .as_deref()
            .unwrap_or_else(|| self.paper.base_font_pt())
    }

    /// Parse the base font size as an f64 value.
    pub fn base_font_value(&self) -> f64 {
        self.effective_font_pt()
            .trim_end_matches("pt")
            .trim_end_matches("px")
            .parse::<f64>()
            .unwrap_or(9.0)
    }

    /// Get the effective grid font size for this layout.
    /// Falls back to base_font_pt if grid_font_pt is not set.
    pub fn grid_font_value(&self) -> f64 {
        self.grid_font_pt
            .as_deref()
            .unwrap_or_else(|| self.effective_font_pt())
            .trim_end_matches("pt")
            .trim_end_matches("px")
            .parse::<f64>()
            .unwrap_or_else(|_| self.base_font_value())
    }

    /// Page background as a Typst color expression, if a valid [`page_fill`] is
    /// set; `None` leaves the page white.
    pub fn page_fill_expr(&self) -> Option<String> {
        self.page_fill.as_deref().and_then(sanitize_color)
    }

    /// Empty grid-cell fill as a Typst color expression, if a valid
    /// [`empty_grid_fill`] is set; `None` keeps the grid's built-in gray.
    pub fn empty_grid_fill_expr(&self) -> Option<String> {
        self.empty_grid_fill.as_deref().and_then(sanitize_color)
    }

    /// Card background as a Typst color expression (defaults to `white`).
    pub fn card_fill_expr(&self) -> String {
        self.card_fill
            .as_deref()
            .and_then(sanitize_color)
            .unwrap_or_else(|| "white".to_string())
    }

    /// Column-gutter override as a Typst length, if a valid [`column_gap`] is
    /// set; `None` leaves the default `_col-gutter` in place.
    pub fn column_gap_expr(&self) -> Option<String> {
        self.column_gap.as_deref().and_then(sanitize_length)
    }

    /// The expression assigned to `_card-gap`: a Typst length, or
    /// `_col-gutter` when unset / `"column"` / invalid.
    pub fn card_gap_expr(&self) -> String {
        match self.card_gap.as_deref().map(str::trim) {
            None => "_col-gutter".to_string(),
            Some(s) if matches!(s.to_ascii_lowercase().as_str(), "column" | "col" | "gutter") => {
                "_col-gutter".to_string()
            }
            Some(s) => sanitize_length(s).unwrap_or_else(|| "_col-gutter".to_string()),
        }
    }

    /// QR code size as a Typst length, honoring [`qr_size`](Self::qr_size) and
    /// falling back to [`crate::qr::DEFAULT_QR_SIZE`] when unset or invalid.
    pub fn qr_size_expr(&self) -> String {
        self.qr_size
            .as_deref()
            .and_then(sanitize_length)
            .unwrap_or_else(|| crate::qr::DEFAULT_QR_SIZE.to_string())
    }

    /// QR caption text size as a Typst length, honoring
    /// [`qr_caption_pt`](Self::qr_caption_pt) and falling back to
    /// [`crate::qr::DEFAULT_CAPTION_SIZE`].
    pub fn qr_caption_size_expr(&self) -> String {
        self.qr_caption_pt
            .as_deref()
            .and_then(sanitize_length)
            .unwrap_or_else(|| crate::qr::DEFAULT_CAPTION_SIZE.to_string())
    }

    /// QR URL text size as a Typst length, honoring [`qr_url_pt`](Self::qr_url_pt)
    /// and falling back to [`crate::qr::DEFAULT_URL_SIZE`].
    pub fn qr_url_size_expr(&self) -> String {
        self.qr_url_pt
            .as_deref()
            .and_then(sanitize_length)
            .unwrap_or_else(|| crate::qr::DEFAULT_URL_SIZE.to_string())
    }
}

/// Sanitize a user-supplied color into a Typst color expression, or `None` if it
/// is not a recognized form. Accepts:
///
/// - hex: `#rgb`, `#rrggbb`, `#rrggbbaa` (the leading `#` is optional)
/// - grayscale: `luma(230)` or `luma(95%)`
/// - a named Typst color (`white`, `silver`, `gray`, `teal`, …)
///
/// Anything else returns `None` so the caller falls back to its default rather
/// than emitting invalid Typst.
pub fn sanitize_color(s: &str) -> Option<String> {
    let t = s.trim();

    // Hex, with or without a leading '#'.
    let hex = t.strip_prefix('#').unwrap_or(t);
    if matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(format!("rgb(\"#{}\")", hex.to_ascii_lowercase()));
    }

    // luma(<number>) or luma(<number>%).
    if let Some(inner) = t.strip_prefix("luma(").and_then(|r| r.strip_suffix(')')) {
        let raw = inner.trim();
        let (num, pct) = match raw.strip_suffix('%') {
            Some(n) => (n.trim(), "%"),
            None => (raw, ""),
        };
        if num.parse::<f64>().is_ok() {
            return Some(format!("luma({num}{pct})"));
        }
    }

    // Named Typst colors (the standard palette).
    const NAMED: &[&str] = &[
        "black", "gray", "silver", "white", "navy", "blue", "aqua", "teal", "eastern", "purple",
        "fuchsia", "maroon", "red", "orange", "yellow", "olive", "green", "lime",
    ];
    let lower = t.to_ascii_lowercase();
    if NAMED.contains(&lower.as_str()) {
        return Some(lower);
    }

    None
}

/// Sanitize a user-supplied length into a Typst length, or `None` if it does not
/// match `<number><unit>` with an allowed absolute/relative unit.
pub fn sanitize_length(s: &str) -> Option<String> {
    let t = s.trim();
    for unit in ["pt", "in", "mm", "cm", "em"] {
        if let Some(num) = t.strip_suffix(unit) {
            let num = num.trim();
            if num.parse::<f64>().is_ok() {
                return Some(format!("{num}{unit}"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_size_dimensions() {
        let (w, h) = PaperSize::Tabloid.dimensions_mm();
        assert!(w > 0.0 && h > 0.0);
        assert!(h > w); // portrait: height > width
    }

    #[test]
    fn test_paper_size_letter_dimensions() {
        let (w, h) = PaperSize::Letter.dimensions_mm();
        assert!(w > 0.0 && h > 0.0);
        assert!(h > w); // portrait
    }

    #[test]
    fn test_paper_size_poster_dimensions() {
        let (w, h) = PaperSize::Poster.dimensions_mm();
        // 20"×30" in portrait basis → 508mm × 762mm
        assert!((w - 508.0).abs() < 1.0);
        assert!((h - 762.0).abs() < 1.0);
    }

    #[test]
    fn test_paper_size_typst_name() {
        assert_eq!(PaperSize::Letter.typst_name(), Some("us-letter"));
        assert_eq!(PaperSize::Tabloid.typst_name(), Some("us-tabloid"));
        assert_eq!(PaperSize::Poster.typst_name(), None);
    }

    #[test]
    fn test_paper_size_dir_name() {
        assert_eq!(PaperSize::Letter.dir_name(), "letter");
        assert_eq!(PaperSize::Legal.dir_name(), "legal");
        assert_eq!(PaperSize::Tabloid.dir_name(), "tabloid");
        assert_eq!(PaperSize::Poster.dir_name(), "poster");
        assert_eq!(PaperSize::Postcard4x6.dir_name(), "postcard");
    }

    #[test]
    fn test_paper_size_description_columns() {
        assert_eq!(
            PaperSize::Letter.description_columns(Orientation::Portrait),
            3
        );
        assert_eq!(
            PaperSize::Letter.description_columns(Orientation::Landscape),
            4
        );
        assert_eq!(
            PaperSize::Legal.description_columns(Orientation::Portrait),
            3
        );
        assert_eq!(
            PaperSize::Tabloid.description_columns(Orientation::Landscape),
            5
        );
        assert_eq!(
            PaperSize::Poster.description_columns(Orientation::Landscape),
            5
        );
    }

    #[test]
    fn test_paper_size_base_font_pt() {
        assert_eq!(PaperSize::Letter.base_font_pt(), "9pt");
        assert_eq!(PaperSize::Poster.base_font_pt(), "10pt");
    }

    #[test]
    fn test_sanitize_color_forms() {
        assert_eq!(
            sanitize_color("#F2F2F2").as_deref(),
            Some("rgb(\"#f2f2f2\")")
        );
        assert_eq!(
            sanitize_color("f2f2f2").as_deref(),
            Some("rgb(\"#f2f2f2\")")
        );
        assert_eq!(sanitize_color("luma(95%)").as_deref(), Some("luma(95%)"));
        assert_eq!(sanitize_color("luma( 230 )").as_deref(), Some("luma(230)"));
        assert_eq!(sanitize_color("white").as_deref(), Some("white"));
        assert_eq!(sanitize_color("Teal").as_deref(), Some("teal"));
        // Rejected: injection, unknown names, malformed hex.
        assert_eq!(sanitize_color("red); #set page(fill: black"), None);
        assert_eq!(sanitize_color("chartreuse"), None);
        assert_eq!(sanitize_color("#12345"), None);
        assert_eq!(sanitize_color("luma(abc)"), None);
    }

    #[test]
    fn test_sanitize_length_forms() {
        assert_eq!(sanitize_length("0.2in").as_deref(), Some("0.2in"));
        assert_eq!(sanitize_length(" 14pt ").as_deref(), Some("14pt"));
        assert_eq!(sanitize_length("3mm").as_deref(), Some("3mm"));
        assert_eq!(sanitize_length("10px"), None);
        assert_eq!(sanitize_length("pt"), None);
    }

    #[test]
    fn test_card_gap_expr_defaults_to_gutter() {
        let mut cfg = LayoutConfig::default();
        assert_eq!(cfg.card_gap_expr(), "_col-gutter");
        cfg.card_gap = Some("column".to_string());
        assert_eq!(cfg.card_gap_expr(), "_col-gutter");
        cfg.card_gap = Some("12pt".to_string());
        assert_eq!(cfg.card_gap_expr(), "12pt");
        cfg.card_gap = Some("bogus".to_string());
        assert_eq!(cfg.card_gap_expr(), "_col-gutter");
    }
}
