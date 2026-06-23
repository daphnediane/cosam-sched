/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Unified print-layout builder.
//!
//! Produces a *single* multi-section Typst document. Every print artifact —
//! schedule grids, description booklets, workshop listings, room signs, and
//! guest panel lists — is one configuration of this builder, selected by
//! [`ContentMode`] (what to draw) plus a [`SectionSplit`]/[`TimeSplit`] pair
//! (how to break it into sections):
//!
//! - [`ContentMode::Both`]: grid on the left half of each section, descriptions
//!   flowing through the remaining columns.
//! - [`ContentMode::GridOnly`]: full-width schedule grid per section.
//! - [`ContentMode::DescriptionOnly`]: multi-column descriptions; page breaks
//!   between sections (`None` split = one continuous flow).
//! - [`ContentMode::PanelList`]: compact name + time + room list (former guest
//!   postcards).
//!
//! Grid-bearing content collapses room/presenter splits to their per-day form (a
//! grid spans a single day): a [`SectionSplit::Room`] section highlights its room
//! column, a [`SectionSplit::Presenter`] section highlights the guest's own cells
//! in the day grid. [`LayoutConfig::double_sided`] pads each section onto an odd
//! page; [`LayoutConfig::header_text`] and [`FooterMode`] drive the banners.

use std::collections::{HashMap, HashSet};

use crate::blocks::banner;
use crate::blocks::grid::{render_schedule_grid, GridRenderConfig};
use crate::blocks::panels::{render_panel_list, render_time_grouped_panels, PanelStyle};
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::config::{ContentMode, FooterMode, LayoutConfig, PanelFilter, SectionSplit, TimeSplit};
use crate::model::{Panel, ScheduleData};
use crate::timegrid::GridLayout;
use crate::typst_gen::preamble;
use schedule_core::value::timezone::{epoch_to_local_iso, local_iso_to_epoch};

/// One renderable section of the document.
struct Section<'a> {
    /// Panels shown in the descriptions/list.
    content_panels: Vec<&'a Panel>,
    /// Panels the grid is built from (the full day for room/presenter sections).
    grid_panels: Vec<&'a Panel>,
    /// Highlight this room's grid column (Room/RoomDay).
    highlight_room: Option<i32>,
    /// Highlight these event cells by panel id (Presenter/PresenterDay).
    highlight_panel_ids: Option<HashSet<String>>,
    /// Running-header left label (2-D entity); empty otherwise.
    left_label: String,
    /// Running-header right label (day, or the 1-D section label); empty for None.
    right_label: String,
    /// Grid corner-cell text.
    corner_label: String,
    /// Start of the visible time window for this section (Unix epoch secs), if any.
    window_start: Option<i64>,
    /// End of the visible time window for this section (Unix epoch secs), if any.
    window_end: Option<i64>,
    /// Override for base font size (e.g., "14pt") for this section.
    /// If None, uses the job's global `base_font_pt` setting.
    base_font_override: Option<String>,
    /// Override for grid font size (e.g., "10pt") for this section.
    /// If None, uses the job's global `grid_font_pt` setting (or base_font_pt).
    grid_font_override: Option<String>,
}

/// Generate the unified layout document.
///
/// Returns a single `(qualifier, typ_source)` pair with an empty qualifier — the
/// whole document (all sections) lives in one file; extract individual pages
/// from the PDF afterward if needed.
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
) -> Vec<(String, String)> {
    let color_mode = config.color_mode;
    let panels = filter_panels(data, data.scheduled_panels(), config.panel_filter);
    if panels.is_empty() {
        return vec![];
    }

    let sections = build_sections(config, data, &panels);
    if sections.is_empty() {
        return vec![];
    }

    let content = &config.content;

    let mut doc = preamble(config, brand);

    // Re-bind `_col-gutter` when the job overrides the column gutter (this must
    // come before `_card-gap`, which may reference the resolved `_col-gutter`).
    if let Some(gap) = config.column_gap_expr() {
        doc.push_str(&format!("#let _col-gutter = {gap}\n"));
    }

    // Panel rendering style (card vs. left-bar). Cards get an explicit inter-card
    // `below` gap (default = the column gutter); the bar style keeps Typst's
    // default block spacing, so non-card jobs emit no extra `#let`.
    let card_gap = config.cards.then(|| {
        doc.push_str(&format!("#let _card-gap = {}\n", config.card_gap_expr()));
        "_card-gap".to_string()
    });
    let panel_style = PanelStyle {
        card: config.cards,
        card_fill: config.card_fill_expr(),
        gap: card_gap,
    };
    let empty_grid_fill = config.empty_grid_fill_expr();
    let dim_conflict = config.dim_conflict;

    // The header bar is always present (fixed top margin); widen the bottom only
    // when a footer is shown (the preamble's bottom margin is tuned for
    // edge-to-edge grids). Geometry `#let`s come from the preamble.
    let bottom = if matches!(config.footer, FooterMode::None) {
        "_page-edge"
    } else {
        "_footer-bottom"
    };
    let page_fill_attr = config
        .page_fill_expr()
        .map(|c| format!("fill: {c}, "))
        .unwrap_or_default();
    // `footer-descent` is the gap between the body and the footer *top*, so it is
    // pinned to 0: the footer block begins at the body bottom and reserves its own
    // bottom margin (`_footer-descent`) via its height (see `banner::footer_context`).
    doc.push_str(&format!(
        "#set page({page_fill_attr}margin: (top: _content-top, bottom: {bottom}, left: _page-edge, right: _page-edge), \
         footer-descent: 0pt)\n",
    ));

    // Footer (selected by FooterMode), set before the header so both `#set page`
    // directives apply document-wide.
    let timestamps = banner::footer_timestamps(&data.meta.modified, &data.meta.generated);
    let site = brand
        .meta
        .site_url
        .as_deref()
        .or(brand.meta.name.as_deref())
        .unwrap_or_default();
    match config.footer {
        FooterMode::Full => doc.push_str(&banner::page_footer(brand, &timestamps, site)),
        FooterMode::TimestampOnly => {
            doc.push_str(&banner::page_footer_timestamps_only(&timestamps))
        }
        FooterMode::SectionPages => {
            doc.push_str(&banner::page_footer_section_pages(&timestamps, site))
        }
        FooterMode::None => {}
    }

    // Header: static for `None`, otherwise a running header fed by `<section>`
    // markers emitted at each section start.
    doc.push_str(&build_header(brand, config));

    // Optional QR code in the bottom-right corner of every page.
    if let Some(url) = config.qr_url.as_deref() {
        if let Some(qr) = crate::qr::qr_page_foreground(
            url,
            config.qr_msg.as_deref(),
            &config.qr_caption_size_expr(),
            &config.qr_url_size_expr(),
            &config.qr_size_expr(),
        ) {
            doc.push_str(&qr);
        } else {
            eprintln!("warning: QR URL could not be encoded (too long?): {url}");
        }
    }

    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            if config.double_sided {
                doc.push_str("#pagebreak(to: \"odd\")\n\n");
            } else {
                doc.push_str("#pagebreak()\n\n");
            }
        }

        if config.content.has_split() {
            doc.push_str(&section_marker(&section.left_label, &section.right_label));
        }

        // Whether to fit the grid to one page (compress + condense). Defaults to
        // on for full-page grids, off otherwise; a job may override either way.
        let grid_fit = config
            .fit_grid
            .unwrap_or(matches!(content, ContentMode::GridOnly { .. }));

        // Wrap section content in a block to scope font overrides
        let has_font_override =
            section.base_font_override.is_some() || section.grid_font_override.is_some();
        if has_font_override {
            doc.push_str("#block[\n");
            // Override base font size and derived font sizes
            if let Some(ref base_font) = section.base_font_override {
                doc.push_str(&format!("#set text(size: {})\n", base_font));
                // Update derived font sizes for this section
                let base_value = base_font
                    .trim_end_matches("pt")
                    .trim_end_matches("px")
                    .parse::<f64>()
                    .unwrap_or(12.0);
                let desc_secondary = (base_value * 0.85).max(6.0);
                let grid_value = section.grid_font_override.as_deref().unwrap_or(base_font);
                let grid_value_num = grid_value
                    .trim_end_matches("pt")
                    .trim_end_matches("px")
                    .parse::<f64>()
                    .unwrap_or(base_value);
                let grid_secondary = (grid_value_num * 0.75).max(5.0);
                doc.push_str(&format!("#let _body-size = {}\n", base_font));
                doc.push_str(&format!(
                    "#let _desc-secondary-size = {}pt\n",
                    desc_secondary
                ));
                doc.push_str(&format!("#let _grid-size = {}\n", grid_value));
                doc.push_str(&format!(
                    "#let _grid-secondary-size = {}pt\n",
                    grid_secondary
                ));
            } else if let Some(ref grid_font) = section.grid_font_override {
                // Only grid font override, still need to update grid sizes
                let grid_value_num = grid_font
                    .trim_end_matches("pt")
                    .trim_end_matches("px")
                    .parse::<f64>()
                    .unwrap_or(12.0);
                let grid_secondary = (grid_value_num * 0.75).max(5.0);
                doc.push_str(&format!("#let _grid-size = {}\n", grid_font));
                doc.push_str(&format!(
                    "#let _grid-secondary-size = {}pt\n",
                    grid_secondary
                ));
            }
        }

        match content {
            ContentMode::GridOnly { .. } => {
                // Apply per-section grid font override if set
                if let Some(ref grid_font) = section.grid_font_override {
                    doc.push_str(&format!("#set text(size: {})\n", grid_font));
                }
                doc.push_str(&render_grid(
                    section,
                    data,
                    color_mode,
                    empty_grid_fill.as_deref(),
                    grid_fit,
                    dim_conflict,
                    CellOptions::from_config(config),
                ));
            }
            ContentMode::Both { .. } => {
                let total_cols =
                    config.effective_columns(config.paper.flyer_columns(config.orientation));
                let grid_cols = total_cols.div_ceil(2);
                let grid_pct = grid_cols as f64 / total_cols as f64 * 100.0;

                // Apply per-section grid font override if set
                if let Some(ref grid_font) = section.grid_font_override {
                    doc.push_str(&format!("#set text(size: {})\n", grid_font));
                }

                doc.push_str(&format!(
                    "#place(top + left, box(width: {:.2}%)[\n",
                    grid_pct
                ));
                doc.push_str(&render_grid(
                    section,
                    data,
                    color_mode,
                    empty_grid_fill.as_deref(),
                    grid_fit,
                    dim_conflict,
                    CellOptions::from_config(config),
                ));
                doc.push_str("])\n");

                doc.push_str(&format!("#columns({}, gutter: _col-gutter)[\n", total_cols));
                for _ in 0..grid_cols {
                    doc.push_str("#colbreak()\n");
                }
                doc.push_str(&render_time_grouped_panels(
                    data,
                    color_mode,
                    &section.content_panels,
                    &panel_style,
                ));
                doc.push_str("]\n");
            }
            ContentMode::DescriptionOnly { .. } => {
                let total_cols =
                    config.effective_columns(config.paper.description_columns(config.orientation));
                doc.push_str(&format!("#columns({}, gutter: _col-gutter)[\n", total_cols));
                doc.push_str(&render_time_grouped_panels(
                    data,
                    color_mode,
                    &section.content_panels,
                    &panel_style,
                ));
                doc.push_str("]\n");
            }
            ContentMode::PanelList { .. } => {
                let total_cols =
                    config.effective_columns(config.paper.description_columns(config.orientation));
                doc.push_str(&format!("#columns({}, gutter: _col-gutter)[\n", total_cols));
                doc.push_str(&render_panel_list(
                    data,
                    color_mode,
                    &section.content_panels,
                ));
                doc.push_str("]\n");
            }
        }

        // Close the block if we opened it
        if has_font_override {
            doc.push_str("]\n");
        }
    }

    vec![(String::new(), doc)]
}

/// Per-job grid-cell content options resolved from the [`LayoutConfig`].
#[derive(Debug, Clone, Copy)]
struct CellOptions {
    show_cost: bool,
    show_duration: bool,
    fit_text: crate::config::FitText,
}

impl CellOptions {
    fn from_config(config: &LayoutConfig) -> Self {
        Self {
            show_cost: config.show_cost(),
            show_duration: config.show_duration(),
            fit_text: config.fit_text,
        }
    }
}

/// Render the schedule grid for a section, applying both highlight kinds.
///
/// When `fit_page` is set (full-page `GridOnly` content) the grid is capped to
/// the page body height so its `1fr` rows compress to fit a single page — a
/// full-day grid would otherwise run off the bottom on smaller paper. The
/// side-by-side `Both` layout passes `false` so the grid keeps its natural
/// height beside the description columns.
fn render_grid(
    section: &Section,
    data: &ScheduleData,
    color_mode: ColorMode,
    empty_fill: Option<&str>,
    fit_page: bool,
    dim_conflict: bool,
    cell: CellOptions,
) -> String {
    // Grid font sizes are global `#let`s from the preamble (`fonts::typst_lets`).
    let mut cfg = GridRenderConfig::full_page("", section.highlight_room);
    cfg.corner_label = section.corner_label.clone();
    cfg.highlight_panel_ids = section.highlight_panel_ids.clone();
    cfg.empty_fill = empty_fill.map(str::to_string);
    cfg.fit_to_page = fit_page;
    cfg.dim_conflict = dim_conflict;
    cfg.show_cost = cell.show_cost;
    cfg.show_duration = cell.show_duration;
    cfg.fit_text = cell.fit_text;
    let layout = GridLayout::compute(
        &section.grid_panels,
        data,
        section.window_start,
        section.window_end,
    );
    render_schedule_grid(&layout, data, color_mode, &cfg)
}

/// Resolve the logo for a layout job to a forward-slash path string, or `None`
/// to suppress the logo.
///
/// - `config.logo` == `None` or `Some("none")` → `None` (no logo)
/// - `config.logo` == `Some(name)` → looked up via [`BrandLogos::resolve_logo`]
/// - When `config.logo` is not set the field defaults to `None`, which means
///   no logo unless the caller has set `Some("brand")` explicitly.
fn resolve_logo(brand: &BrandConfig, config: &LayoutConfig) -> Option<String> {
    let name = config.logo.as_deref()?;
    if name.eq_ignore_ascii_case("none") {
        return None;
    }
    brand
        .logos
        .resolve_logo(name)
        .and_then(|p| p.into_os_string().into_string().ok())
        .map(|s| s.replace('\\', "/"))
}

/// Build the page header directive for the chosen split.
fn build_header(brand: &BrandConfig, config: &LayoutConfig) -> String {
    // Resolve the logo path once; all banner variants accept Option<&str>.
    let logo_str = resolve_logo(brand, config);
    let logo = logo_str.as_deref();

    if !config.content.has_split() {
        // Static header; header_text on the right.
        return banner::page_header(brand, logo, None, config.header_text.as_deref());
    }
    let right = running_field("right");
    if config.content.is_two_dim() {
        // Entity left, day right; header_text omitted (both slots taken).
        banner::page_header_running_split(brand, logo, &running_field("left"), &right)
    } else if let Some(text) = config.header_text.as_deref() {
        // header_text literal on the left, running section label on the right.
        banner::page_header_running_split(brand, logo, &escape_literal(text), &right)
    } else {
        // Logo (if any) left, running section label right.
        banner::page_header_running(brand, logo, &right)
    }
}

/// `#context` expression reading `field` ("left"/"right") from the latest
/// `<section>` marker on or before the current page.
fn running_field(field: &str) -> String {
    format!(
        "#context {{\n    \
           let _m = query(<section>).filter(m => m.location().page() <= here().page())\n    \
           if _m.len() > 0 {{ _m.last().value.{field} }}\n  \
         }}",
    )
}

/// Invisible per-section marker carrying the header's left/right labels.
fn section_marker(left: &str, right: &str) -> String {
    format!(
        "#metadata((left: \"{}\", right: \"{}\")) <section>\n",
        escape_literal(left),
        escape_literal(right),
    )
}

/// Escape a string for embedding inside a Typst double-quoted literal.
fn escape_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Restrict the panel set to those matching the filter.
fn filter_panels<'a>(
    data: &ScheduleData,
    panels: Vec<&'a Panel>,
    filter: PanelFilter,
) -> Vec<&'a Panel> {
    match filter {
        PanelFilter::All => panels,
        PanelFilter::Workshops => panels
            .into_iter()
            .filter(|p| is_workshop(data, p))
            .collect(),
        PanelFilter::Premium => panels.into_iter().filter(|p| p.is_premium).collect(),
    }
}

/// Whether a panel's type is a workshop.
fn is_workshop(data: &ScheduleData, panel: &Panel) -> bool {
    panel
        .panel_type
        .as_ref()
        .and_then(|pt| data.panel_types.get(pt.as_str()))
        .is_some_and(|pt| pt.is_workshop)
}

/// The identity and framing of a section, without its panels: a label, an
/// optional visible time window (epoch-second bounds used to clamp the grid to
/// the section boundary), and optional per-section font-size overrides. Time
/// splits build these first, then pair each with its bucket of panels via
/// [`SectionKey::with_panels`].
struct SectionKey {
    label: String,
    window_start: Option<i64>,
    window_end: Option<i64>,
    base_font: Option<String>,
    grid_font: Option<String>,
}

impl SectionKey {
    /// A section with no time-window clamp and no per-section font overrides.
    fn new(label: String) -> Self {
        Self {
            label,
            window_start: None,
            window_end: None,
            base_font: None,
            grid_font: None,
        }
    }

    /// Set the visible time window (epoch-second bounds) that clamps the grid.
    fn window(mut self, start: Option<i64>, end: Option<i64>) -> Self {
        self.window_start = start;
        self.window_end = end;
        self
    }

    /// Set the per-section base/grid font-size overrides.
    fn fonts(mut self, base: Option<String>, grid: Option<String>) -> Self {
        self.base_font = base;
        self.grid_font = grid;
        self
    }

    /// Pair this key with the panels that fall in it.
    fn with_panels<'a>(self, panels: Vec<&'a Panel>) -> TimedSection<'a> {
        TimedSection { key: self, panels }
    }
}

/// A [`SectionKey`] paired with the panels that fall in its label/window.
struct TimedSection<'a> {
    key: SectionKey,
    panels: Vec<&'a Panel>,
}

/// Flatten `panels` into time-labeled slices for `time`, using the export's
/// precomputed day/half-day timelines (epoch ranges + labels).
fn time_sections<'a>(
    time: &TimeSplit,
    panels: &[&'a Panel],
    data: &ScheduleData,
) -> Vec<TimedSection<'a>> {
    match time {
        TimeSplit::Day => day_sections(panels, data),
        TimeSplit::HalfDay => half_day_sections(panels, data),
        TimeSplit::Timeline => split_on_timeline(panels, data),
        TimeSplit::Custom(ct) => split_on_custom_timeline(panels, data, ct),
    }
}

/// Bucket panels by their precomputed `day_key` (which already encodes the
/// late-night rollover), preserving the chronological order of the slice.
fn bucket_by_day_key<'a>(panels: &[&'a Panel]) -> HashMap<&'a str, Vec<&'a Panel>> {
    let mut by_key: HashMap<&'a str, Vec<&'a Panel>> = HashMap::new();
    for p in panels {
        if let Some(k) = p.day_key.as_deref() {
            by_key.entry(k).or_default().push(p);
        }
    }
    by_key
}

/// One section per content day, from `data.day_timeline`. A day has no grid
/// clipping (`win = None`); content is bucketed by `day_key`.
fn day_sections<'a>(panels: &[&'a Panel], data: &ScheduleData) -> Vec<TimedSection<'a>> {
    let mut by_key = bucket_by_day_key(panels);
    data.day_timeline
        .iter()
        .filter_map(|span| {
            let ps = by_key.remove(span.date.as_str())?;
            (!ps.is_empty()).then(|| SectionKey::new(span.label.clone()).with_panels(ps))
        })
        .collect()
}

/// AM/PM sections from `data.half_day_timeline`. A date split into both halves
/// emits the timeline's `"<Day> AM"`/`"<Day> PM"` labels and clamps each grid to
/// the half's boundary (the exporter already clamps these to local noon when a
/// session crosses it). A single-half date is one undivided section.
fn half_day_sections<'a>(panels: &[&'a Panel], data: &ScheduleData) -> Vec<TimedSection<'a>> {
    let mut by_key = bucket_by_day_key(panels);
    let mut out = vec![];
    for day in &data.day_timeline {
        let Some(day_panels) = by_key.remove(day.date.as_str()) else {
            continue;
        };
        let halves: Vec<_> = data
            .half_day_timeline
            .iter()
            .filter(|s| s.date == day.date)
            .collect();
        match halves.as_slice() {
            [am, pm] => {
                // Split at the PM boundary. When a session crosses noon both spans
                // clamp there; otherwise no session lies in the gap, so the exact
                // split point is immaterial.
                let boundary = pm.start_epoch;
                let (am_p, pm_p): (Vec<_>, Vec<_>) = day_panels
                    .into_iter()
                    .partition(|p| p.start_epoch.is_some_and(|s| s < boundary));
                if !am_p.is_empty() {
                    out.push(
                        SectionKey::new(am.label.clone())
                            .window(None, Some(am.end_epoch))
                            .with_panels(am_p),
                    );
                }
                if !pm_p.is_empty() {
                    out.push(
                        SectionKey::new(pm.label.clone())
                            .window(Some(pm.start_epoch), None)
                            .with_panels(pm_p),
                    );
                }
            }
            [single] => out.push(SectionKey::new(single.label.clone()).with_panels(day_panels)),
            _ => out.push(SectionKey::new(day.label.clone()).with_panels(day_panels)),
        }
    }
    out
}

/// Split panels into sections using the schedule's timeline entries as boundaries.
///
/// Timeline entries (SPLIT panels) carry a `start_time`; each one opens a new
/// section whose label is the entry's name. Panels that fall before the first
/// timeline entry in a day are grouped into a section named after the day itself.
///
/// Each returned section includes the window `[win_start, win_end)` that defines
/// the visible time range, so the grid can clamp and mark panels that span the
/// boundary.
fn split_on_timeline<'a>(panels: &[&'a Panel], data: &ScheduleData) -> Vec<TimedSection<'a>> {
    let tz = data.meta.timezone.as_str();
    // Timeline boundaries as (local date, start epoch, label), sorted by epoch.
    let mut tl_boundaries: Vec<(String, i64, &str)> = data
        .timeline
        .iter()
        .filter_map(|t| {
            let e = t.start_epoch?;
            let date = epoch_to_local_iso(e, tz).get(..10)?.to_string();
            Some((date, e, t.name.as_str()))
        })
        .collect();
    tl_boundaries.sort_by_key(|(_, e, _)| *e);

    let mut by_key = bucket_by_day_key(panels);
    let mut out: Vec<TimedSection<'a>> = vec![];

    for day in &data.day_timeline {
        let Some(day_panels) = by_key.remove(day.date.as_str()) else {
            continue;
        };
        // Boundaries that fall on this calendar day.
        let day_boundaries: Vec<(i64, &str)> = tl_boundaries
            .iter()
            .filter(|(d, _, _)| d.as_str() == day.date.as_str())
            .map(|(_, e, name)| (*e, *name))
            .collect();

        if day_boundaries.is_empty() {
            out.push(SectionKey::new(day.label.clone()).with_panels(day_panels));
            continue;
        }

        // Assign each panel to the latest boundary whose start ≤ panel start.
        let section_label_for = |panel: &&'a Panel| -> String {
            let Some(s) = panel.start_epoch else {
                return day.label.clone();
            };
            day_boundaries
                .iter()
                .rev()
                .find(|(b, _)| *b <= s)
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| day.label.clone())
        };

        // Day-label catch-all first (window [None, first_boundary)), then each
        // boundary in order (window [boundary, next_boundary)).
        let mut section_keys: Vec<SectionKey> =
            vec![SectionKey::new(day.label.clone()).window(None, Some(day_boundaries[0].0))];
        for (i, (bstart, name)) in day_boundaries.iter().enumerate() {
            let key = name.to_string();
            let win_end = day_boundaries.get(i + 1).map(|(next, _)| *next);
            if !section_keys.iter().any(|k| k.label == key) {
                section_keys.push(SectionKey::new(key).window(Some(*bstart), win_end));
            }
        }

        let mut buckets: HashMap<String, Vec<&'a Panel>> = HashMap::new();
        for panel in &day_panels {
            buckets
                .entry(section_label_for(panel))
                .or_default()
                .push(panel);
        }

        for key in section_keys {
            if let Some(bucket) = buckets.remove(&key.label) {
                if !bucket.is_empty() {
                    out.push(key.with_panels(bucket));
                }
            }
        }
    }

    out
}

/// Split panels into sections using a custom timeline's slots as boundaries.
///
/// Panels that fall before the first slot are handled based on whether that
/// first slot has an explicit `end_time`:
/// - If the first slot is **windowed** (has `end_time`), panels before it are
///   excluded — the slot represents a specific time window, not "from the start".
/// - If the first slot is **open-ended** (no `end_time`), panels before it get
///   their own section labelled `"Before <first_label>"`, so nothing is lost.
///
/// Slot windows chain globally across day boundaries: a slot with no `end_time`
/// runs until the next slot's start time regardless of the calendar date.
fn split_on_custom_timeline<'a>(
    panels: &[&'a Panel],
    data: &ScheduleData,
    custom_timeline: &crate::config::CustomTimeline,
) -> Vec<TimedSection<'a>> {
    use crate::config::CustomTimeSlot;

    let tz = data.meta.timezone.as_str();
    // Schedule range as local-wall-clock ISO, for loose-time resolution.
    let (Some(sched_start), Some(sched_end)) = (
        (data.meta.start_epoch != 0).then(|| epoch_to_local_iso(data.meta.start_epoch, tz)),
        (data.meta.end_epoch != 0).then(|| epoch_to_local_iso(data.meta.end_epoch, tz)),
    ) else {
        // No schedule range available — fall back to a plain day split.
        return day_sections(panels, data);
    };
    let to_epoch = |dt: chrono::NaiveDateTime| -> Option<i64> {
        local_iso_to_epoch(&dt.format("%Y-%m-%dT%H:%M:%S").to_string(), tz)
    };

    // Slots resolved to epochs: (start, explicit_end_opt, label), sorted by start.
    let mut all_slots: Vec<(i64, Option<i64>, &str)> = custom_timeline
        .slots
        .iter()
        .filter_map(|s: &CustomTimeSlot| {
            let start = to_epoch(crate::time_fmt::parse_loose_datetime(
                &s.time,
                &sched_start,
                &sched_end,
            )?)?;
            let end = s
                .end_time
                .as_deref()
                .and_then(|e| crate::time_fmt::parse_loose_datetime(e, &sched_start, &sched_end))
                .and_then(to_epoch);
            Some((start, end, s.label.as_str()))
        })
        .collect();
    all_slots.sort_by_key(|(start, _, _)| *start);

    // Effective end per slot: explicit end, else next slot's start, else open.
    let slot_windows: Vec<(i64, Option<i64>, &str)> = all_slots
        .iter()
        .enumerate()
        .map(|(i, (start, end, label))| {
            let effective_end = end.or_else(|| all_slots.get(i + 1).map(|(ns, _, _)| *ns));
            (*start, effective_end, *label)
        })
        .collect();

    if slot_windows.is_empty() {
        return vec![];
    }

    // A windowed first slot (explicit end) excludes earlier panels; an open-ended
    // first slot gives them a "Before <label>" catch-all.
    let first_slot_is_windowed = all_slots[0].1.is_some();
    let before_label: Option<String> = if first_slot_is_windowed {
        None
    } else {
        let first_label = all_slots[0].2;
        (!first_label.is_empty()).then(|| format!("Before {}", first_label))
    };
    let first_slot_start = all_slots[0].0;

    let section_label_for = |panel: &&'a Panel| -> Option<String> {
        let s = panel.start_epoch?;
        if s < first_slot_start {
            return before_label.clone();
        }
        slot_windows
            .iter()
            .rev()
            .find(|(slot_start, slot_end, _)| *slot_start <= s && slot_end.is_none_or(|e| s < e))
            .map(|(_, _, label)| label.to_string())
    };

    let mut buckets: HashMap<String, Vec<&'a Panel>> = HashMap::new();
    for panel in panels {
        if let Some(label) = section_label_for(panel) {
            buckets.entry(label).or_default().push(panel);
        }
        // Panels outside all slot windows are silently excluded.
    }

    let mut out: Vec<TimedSection<'a>> = vec![];

    if let Some(ref bl) = before_label {
        if let Some(bucket) = buckets.remove(bl.as_str()) {
            if !bucket.is_empty() {
                out.push(
                    SectionKey::new(bl.clone())
                        .window(None, Some(first_slot_start))
                        .with_panels(bucket),
                );
            }
        }
    }

    for (win_start, win_end, label) in &slot_windows {
        if label.is_empty() {
            continue;
        }
        if let Some(bucket) = buckets.remove(*label) {
            if !bucket.is_empty() {
                // Look up font overrides from the custom timeline slot
                let (base_font, grid_font) = custom_timeline
                    .slots
                    .iter()
                    .find(|s| s.label == *label)
                    .map(|s| (s.base_font_pt.clone(), s.grid_font_pt.clone()))
                    .unwrap_or((None, None));
                out.push(
                    SectionKey::new(label.to_string())
                        .window(Some(*win_start), *win_end)
                        .fonts(base_font, grid_font)
                        .with_panels(bucket),
                );
            }
        }
    }

    out
}

/// Return panels from `all_panels` that start before `window_start` but end
/// after it — i.e. panels that were not assigned to this section (because they
/// started earlier) but whose time range overlaps into the window.  These are
/// added to `grid_panels` only so the grid can show them with a truncated-start
/// visual; they are deliberately excluded from `content_panels` to avoid
/// duplicating descriptions in sections where the panel did not begin.
fn spanning_into<'a>(all_panels: &[&'a Panel], window_start: i64) -> Vec<&'a Panel> {
    all_panels
        .iter()
        .copied()
        .filter(|p| match (p.start_epoch, p.end_epoch) {
            // Started before the window and ends after it begins.
            (Some(s), Some(e)) => s < window_start && e > window_start,
            _ => false,
        })
        .collect()
}

/// Grid corner-cell label (above the time column). The section's identity (day /
/// timeline block / room / presenter) is already shown in the running banner, so
/// the corner just labels the time column rather than repeating that name — a
/// long timeline label like "Saturday Afternoon" otherwise wraps *and* widens the
/// whole time column to fit it.
const TIME_CORNER_LABEL: &str = "Time";

/// Build the document's sections for the configured split.
fn build_sections<'a>(
    config: &LayoutConfig,
    data: &ScheduleData,
    panels: &[&'a Panel],
) -> Vec<Section<'a>> {
    match (config.content.section_split(), config.content.time_split()) {
        (None, None) => vec![Section {
            content_panels: panels.to_vec(),
            grid_panels: panels.to_vec(),
            highlight_room: None,
            highlight_panel_ids: None,
            left_label: String::new(),
            right_label: String::new(),
            corner_label: String::new(),
            window_start: None,
            window_end: None,
            base_font_override: None,
            grid_font_override: None,
        }],

        (None, Some(time)) => time_sections(&time, panels, data)
            .into_iter()
            .map(
                |TimedSection {
                     key:
                         SectionKey {
                             label,
                             window_start: win_start,
                             window_end: win_end,
                             base_font,
                             grid_font,
                         },
                     panels: time_panels,
                 }| {
                    let grid_panels = if let Some(ws) = win_start {
                        let mut gp = spanning_into(panels, ws);
                        gp.extend_from_slice(&time_panels);
                        gp
                    } else {
                        time_panels.clone()
                    };
                    Section {
                        content_panels: time_panels,
                        grid_panels,
                        highlight_room: None,
                        highlight_panel_ids: None,
                        left_label: String::new(),
                        right_label: label.clone(),
                        corner_label: TIME_CORNER_LABEL.to_string(),
                        window_start: win_start,
                        window_end: win_end,
                        base_font_override: base_font,
                        grid_font_override: grid_font,
                    }
                },
            )
            .collect(),

        (Some(SectionSplit::Room), None) => data
            .sorted_rooms()
            .iter()
            .filter_map(|room| {
                let room_panels: Vec<&Panel> = panels
                    .iter()
                    .copied()
                    .filter(|p| p.room_ids.contains(&room.uid))
                    .collect();
                if room_panels.is_empty() {
                    return None;
                }
                let name = room_name(room);
                Some(Section {
                    content_panels: room_panels.clone(),
                    grid_panels: room_panels,
                    highlight_room: Some(room.uid),
                    highlight_panel_ids: None,
                    left_label: String::new(),
                    right_label: name.clone(),
                    corner_label: TIME_CORNER_LABEL.to_string(),
                    window_start: None,
                    window_end: None,
                    base_font_override: None,
                    grid_font_override: None,
                })
            })
            .collect(),

        (Some(SectionSplit::Room), Some(time)) => data
            .sorted_rooms()
            .iter()
            .flat_map(|room| {
                let name = room_name(room);
                time_sections(&time, panels, data)
                    .into_iter()
                    .filter_map(
                        move |TimedSection {
                                  key:
                                      SectionKey {
                                          label: time_label,
                                          window_start: win_start,
                                          window_end: win_end,
                                          base_font,
                                          grid_font,
                                      },
                                  panels: time_panels,
                              }| {
                            let room_panels: Vec<&Panel> = time_panels
                                .iter()
                                .copied()
                                .filter(|p| p.room_ids.contains(&room.uid))
                                .collect();
                            if room_panels.is_empty() {
                                return None;
                            }
                            // grid_panels: time section panels + spanning ones for this room.
                            let grid_panels = if let Some(ws) = win_start {
                                let mut gp: Vec<&Panel> = spanning_into(panels, ws)
                                    .into_iter()
                                    .filter(|p| p.room_ids.contains(&room.uid))
                                    .collect();
                                gp.extend_from_slice(&time_panels);
                                gp
                            } else {
                                time_panels.clone()
                            };
                            Some(Section {
                                content_panels: room_panels,
                                grid_panels,
                                highlight_room: Some(room.uid),
                                highlight_panel_ids: None,
                                left_label: name.clone(),
                                right_label: time_label.clone(),
                                corner_label: TIME_CORNER_LABEL.to_string(),
                                window_start: win_start,
                                window_end: win_end,
                                base_font_override: base_font,
                                grid_font_override: grid_font,
                            })
                        },
                    )
                    .collect::<Vec<_>>()
            })
            .collect(),

        (Some(SectionSplit::Presenter), None) => data
            .presenters
            .iter()
            .filter(|p| postcard_rank_eligible(&p.rank))
            .filter_map(|presenter| {
                // Match by the presenter's authoritative panel-id list (groups
                // subsume members), not a name string-match.
                let pid: HashSet<&str> = presenter.panel_ids.iter().map(String::as_str).collect();
                let pres_panels: Vec<&Panel> = panels
                    .iter()
                    .copied()
                    .filter(|p| pid.contains(p.id.as_str()))
                    .collect();
                if pres_panels.is_empty() {
                    return None;
                }
                Some(Section {
                    content_panels: pres_panels.clone(),
                    grid_panels: pres_panels,
                    highlight_room: None,
                    highlight_panel_ids: None,
                    left_label: String::new(),
                    right_label: presenter.name.clone(),
                    corner_label: TIME_CORNER_LABEL.to_string(),
                    window_start: None,
                    window_end: None,
                    base_font_override: None,
                    grid_font_override: None,
                })
            })
            .collect(),

        (Some(SectionSplit::Presenter), Some(time)) => data
            .presenters
            .iter()
            .filter(|p| postcard_rank_eligible(&p.rank))
            .flat_map(|presenter| {
                // Match by the presenter's authoritative panel-id list (a group's
                // list subsumes its members), not a name string-match — so a group
                // section highlights its members' panels too.
                let pid: HashSet<&str> = presenter.panel_ids.iter().map(String::as_str).collect();
                time_sections(&time, panels, data)
                    .into_iter()
                    .filter_map(
                        move |TimedSection {
                                  key:
                                      SectionKey {
                                          label: time_label,
                                          window_start: win_start,
                                          window_end: win_end,
                                          base_font,
                                          grid_font,
                                      },
                                  panels: time_panels,
                              }| {
                            let pres_panels: Vec<&Panel> = time_panels
                                .iter()
                                .copied()
                                .filter(|p| pid.contains(p.id.as_str()))
                                .collect();
                            // Days this guest has no panels: skip unless
                            // `matching_only = false`, which still shows the day —
                            // the full day grid with nothing highlighted.
                            if pres_panels.is_empty() && config.matching_only.unwrap_or(true) {
                                return None;
                            }
                            // grid_panels: the full day grid (everyone) plus panels
                            // spanning into the window. The guest's own panels, if
                            // any, are highlighted; on a non-matching day none are.
                            let grid_panels = if let Some(ws) = win_start {
                                let mut gp = spanning_into(panels, ws);
                                gp.extend_from_slice(&time_panels);
                                gp
                            } else {
                                time_panels.clone()
                            };
                            let highlight = (!pres_panels.is_empty()).then(|| {
                                pres_panels
                                    .iter()
                                    .map(|p| p.id.clone())
                                    .collect::<HashSet<String>>()
                            });
                            Some(Section {
                                content_panels: pres_panels,
                                grid_panels,
                                highlight_room: None,
                                highlight_panel_ids: highlight,
                                left_label: presenter.name.clone(),
                                right_label: time_label.clone(),
                                corner_label: TIME_CORNER_LABEL.to_string(),
                                window_start: win_start,
                                window_end: win_end,
                                base_font_override: base_font,
                                grid_font_override: grid_font,
                            })
                        },
                    )
                    .collect::<Vec<_>>()
            })
            .collect(),
    }
}

/// Room display name (long preferred, else short).
fn room_name(room: &crate::model::Room) -> String {
    if !room.long_name.is_empty() {
        room.long_name.clone()
    } else {
        room.short_name.clone()
    }
}

/// Whether the presenter rank qualifies for a presenter section.
///
/// Mirrors `PresenterRank` priority ≤ 3: `guest`, `judge`, `staff`, and any
/// `invited_*` variant (including the default `invited_panelist`). Panelists and
/// fan-panelists are excluded.
fn postcard_rank_eligible(rank: &str) -> bool {
    matches!(rank, "guest" | "judge" | "staff") || rank.starts_with("invited") || rank.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LayoutConfig, PaperSize};
    use crate::model::{DaySpan, Meta, Panel, Presenter, Room, ScheduleData};

    fn empty_schedule() -> ScheduleData {
        ScheduleData {
            meta: Meta {
                title: "T".into(),
                ..Meta::default()
            },
            ..ScheduleData::default()
        }
    }

    fn panel(id: &str, day_hour: &str, room: i32, presenter: &str) -> Panel {
        // Tests use an empty meta timezone, so wall-clock is interpreted as UTC.
        let epoch = chrono::NaiveDateTime::parse_from_str(
            &format!("2026-06-{day_hour}:00"),
            "%Y-%m-%dT%H:%M",
        )
        .unwrap()
        .and_utc()
        .timestamp();
        Panel {
            id: id.into(),
            base_id: id.into(),
            name: format!("Panel {id}"),
            room_ids: vec![room],
            start_epoch: Some(epoch),
            end_epoch: Some(epoch),
            // day_key is the calendar date (no overnight panels in these fixtures).
            day_key: Some(format!("2026-06-{}", &day_hour[..2])),
            presenters: if presenter.is_empty() {
                vec![]
            } else {
                vec![presenter.to_string()]
            },
            ..Panel::default()
        }
    }

    /// A day-timeline span for `date` (`"2026-06-DD"`) labelled by its weekday.
    fn day_span(date: &str) -> DaySpan {
        let weekday = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .unwrap()
            .format("%A")
            .to_string();
        DaySpan {
            label: weekday,
            date: date.into(),
            ..DaySpan::default()
        }
    }

    fn two_day_schedule() -> ScheduleData {
        let mut d = empty_schedule();
        d.rooms = vec![
            Room {
                uid: 1,
                short_name: "A".into(),
                long_name: "Salon A".into(),
                sort_key: 0,
                ..Room::default()
            },
            Room {
                uid: 2,
                short_name: "B".into(),
                long_name: "Salon B".into(),
                sort_key: 1,
                ..Room::default()
            },
        ];
        d.presenters = vec![Presenter {
            name: "Ada".into(),
            rank: "guest".into(),
            // Authoritative panel list (matching is by id, not name).
            panel_ids: vec!["P1".into(), "P3".into()],
            ..Presenter::default()
        }];
        d.panels = vec![
            panel("P1", "26T09", 1, "Ada"), // Fri AM, room A, Ada
            panel("P2", "26T14", 2, ""),    // Fri PM, room B
            panel("P3", "27T10", 1, "Ada"), // Sat AM, room A, Ada
        ];
        d.day_timeline = vec![day_span("2026-06-26"), day_span("2026-06-27")];
        d
    }

    fn cfg(content: ContentMode) -> LayoutConfig {
        LayoutConfig {
            content,
            ..LayoutConfig::default()
        }
    }

    #[test]
    fn test_generate_empty() {
        let out = generate(
            &empty_schedule(),
            &BrandConfig::default(),
            &LayoutConfig::default(),
        );
        assert!(out.is_empty());
    }

    #[test]
    fn test_sections_day() {
        let d = two_day_schedule();
        let c = cfg(ContentMode::DescriptionOnly {
            section: None,
            time: Some(TimeSplit::Day),
        });
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::All);
        let secs = build_sections(&c, &d, &panels);
        assert_eq!(secs.len(), 2); // two days
    }

    #[test]
    fn test_sections_room_day_uses_full_day_grid() {
        let d = two_day_schedule();
        let c = cfg(ContentMode::Both {
            section: Some(SectionSplit::Room),
            time: TimeSplit::Day,
        });
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::All);
        let secs = build_sections(&c, &d, &panels);
        // Room A: Fri + Sat (2), Room B: Fri (1) => 3 sections.
        assert_eq!(secs.len(), 3);
        let a_fri = &secs[0];
        assert_eq!(a_fri.highlight_room, Some(1));
        assert_eq!(a_fri.content_panels.len(), 1); // only room A's panel
        assert_eq!(a_fri.grid_panels.len(), 2); // full Friday grid (A + B)
        assert_eq!(a_fri.left_label, "Salon A");
    }

    #[test]
    fn test_sections_presenter_day_with_grid() {
        let d = two_day_schedule();
        let c = cfg(ContentMode::Both {
            section: Some(SectionSplit::Presenter),
            time: TimeSplit::Day,
        });
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::All);
        let secs = build_sections(&c, &d, &panels);
        // Ada appears Fri + Sat => 2 sections.
        assert_eq!(secs.len(), 2);
        let fri = &secs[0];
        assert!(fri.highlight_panel_ids.as_ref().unwrap().contains("P1"));
        assert_eq!(fri.grid_panels.len(), 2); // full Friday grid
        assert_eq!(fri.left_label, "Ada");
    }

    /// Bea is scheduled only on Friday; Saturday exists (Ada is on it).
    fn schedule_with_friday_only_presenter() -> ScheduleData {
        let mut d = two_day_schedule();
        d.presenters.push(Presenter {
            name: "Bea".into(),
            rank: "guest".into(),
            panel_ids: vec!["P4".into()],
            ..Presenter::default()
        });
        d.panels.push(panel("P4", "26T11", 2, "Bea")); // Fri only
        d
    }

    #[test]
    fn test_sections_presenter_day_matching_only_false() {
        // With matching_only = false, Bea still gets a Saturday section: the full
        // Saturday grid (everyone's panels) with nothing highlighted.
        let d = schedule_with_friday_only_presenter();
        let c = LayoutConfig {
            content: ContentMode::GridOnly {
                section: Some(SectionSplit::Presenter),
                time: TimeSplit::Day,
            },
            matching_only: Some(false),
            ..LayoutConfig::default()
        };
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::All);
        let secs = build_sections(&c, &d, &panels);

        // Ada: Fri+Sat (2) + Bea: Fri + Sat (2) = 4 sections.
        let bea: Vec<_> = secs.iter().filter(|s| s.left_label == "Bea").collect();
        assert_eq!(bea.len(), 2);
        let bea_sat = bea
            .iter()
            .find(|s| s.right_label == "Saturday")
            .expect("Bea should have a Saturday section");
        // Full Saturday grid is drawn (P3 is the only Saturday panel)...
        assert_eq!(bea_sat.grid_panels.len(), 1);
        // ...but nothing is highlighted, since Bea has no Saturday panel.
        assert!(bea_sat.highlight_panel_ids.is_none());

        // Bea's Friday section highlights her own panel within the full grid.
        let bea_fri = bea.iter().find(|s| s.right_label == "Friday").unwrap();
        assert!(bea_fri
            .highlight_panel_ids
            .as_ref()
            .is_some_and(|h| h.contains("P4")));
    }

    #[test]
    fn test_sections_presenter_day_no_empty_grids_by_default() {
        // Without the flag, a presenter's empty day is skipped (current behavior).
        let d = schedule_with_friday_only_presenter();
        let c = cfg(ContentMode::GridOnly {
            section: Some(SectionSplit::Presenter),
            time: TimeSplit::Day,
        });
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::All);
        let secs = build_sections(&c, &d, &panels);
        let bea: Vec<_> = secs.iter().filter(|s| s.left_label == "Bea").collect();
        assert_eq!(bea.len(), 1); // Friday only
    }

    #[test]
    fn test_sections_group_highlights_member_panels() {
        // A group matches by its panel_ids (which subsume members), not by name —
        // even when the group name appears on no panel's presenter list.
        let mut d = two_day_schedule();
        d.presenters.push(Presenter {
            name: "The Group".into(),
            rank: "guest".into(),
            is_group: true,
            members: vec!["Ada".into()],
            // Subsumes Ada's Friday panel P1, though "The Group" credits nothing.
            panel_ids: vec!["P1".into()],
            ..Presenter::default()
        });

        let c = cfg(ContentMode::GridOnly {
            section: Some(SectionSplit::Presenter),
            time: TimeSplit::Day,
        });
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::All);
        let secs = build_sections(&c, &d, &panels);
        let group: Vec<_> = secs
            .iter()
            .filter(|s| s.left_label == "The Group")
            .collect();
        // One section (Friday), highlighting the member's panel.
        assert_eq!(group.len(), 1);
        assert!(group[0]
            .highlight_panel_ids
            .as_ref()
            .is_some_and(|h| h.contains("P1")));
    }

    #[test]
    fn test_sections_presenter_listing_spans_days() {
        let d = two_day_schedule();
        // Presenter split with non-grid content stays multi-day per presenter.
        let c = cfg(ContentMode::PanelList {
            section: Some(SectionSplit::Presenter),
            time: None,
        });
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::All);
        let secs = build_sections(&c, &d, &panels);
        assert_eq!(secs.len(), 1); // one section for Ada, both days
        assert_eq!(secs[0].content_panels.len(), 2);
    }

    #[test]
    fn test_panel_filter_premium() {
        let mut d = two_day_schedule();
        d.panels[0].is_premium = true;
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::Premium);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].id, "P1");
    }

    #[test]
    fn test_flyer_columns_split() {
        assert_eq!(
            PaperSize::Letter.flyer_columns(crate::config::Orientation::Landscape),
            4
        );
        assert_eq!(
            PaperSize::Legal.flyer_columns(crate::config::Orientation::Landscape),
            6
        );
    }

    #[test]
    fn test_rank_eligible() {
        assert!(postcard_rank_eligible("guest"));
        assert!(postcard_rank_eligible("invited_author"));
        assert!(!postcard_rank_eligible("panelist"));
    }
}
