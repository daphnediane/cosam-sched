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

use std::collections::HashSet;

use crate::blocks::banner;
use crate::blocks::grid::{render_schedule_grid, GridRenderConfig};
use crate::blocks::panels::{render_panel_list, render_time_grouped_panels, PanelStyle};
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::config::{ContentMode, FooterMode, LayoutConfig, PanelFilter, SectionSplit, TimeSplit};
use crate::model::{
    meta_end_iso, meta_start_iso, panel_end_iso, panel_start_iso, timeline_start_iso, Panel,
    ScheduleData, TimelineEntry,
};
use crate::timegrid::GridLayout;
use crate::typst_gen::{make_day_label, preamble};

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
    /// Start of the visible time window for this section (ISO 8601), if any.
    window_start: Option<String>,
    /// End of the visible time window for this section (ISO 8601), if any.
    window_end: Option<String>,
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
    doc.push_str(&format!(
        "#set page({page_fill_attr}margin: (top: _content-top, bottom: {bottom}, left: _page-edge, right: _page-edge), \
         footer-descent: _footer-descent)\n",
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
        FooterMode::None => {}
    }

    // Header: static for `None`, otherwise a running header fed by `<section>`
    // markers emitted at each section start.
    doc.push_str(&build_header(brand, config));

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

        match content {
            ContentMode::GridOnly { .. } => {
                doc.push_str(&render_grid(
                    section,
                    data,
                    color_mode,
                    empty_grid_fill.as_deref(),
                ));
            }
            ContentMode::Both { .. } => {
                let total_cols =
                    config.effective_columns(config.paper.flyer_columns(config.orientation));
                let grid_cols = total_cols.div_ceil(2);
                let grid_pct = grid_cols as f64 / total_cols as f64 * 100.0;

                doc.push_str(&format!(
                    "#place(top + left, box(width: {:.2}%)[\n",
                    grid_pct
                ));
                doc.push_str(&render_grid(
                    section,
                    data,
                    color_mode,
                    empty_grid_fill.as_deref(),
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
    }

    vec![(String::new(), doc)]
}

/// Render the schedule grid for a section, applying both highlight kinds.
fn render_grid(
    section: &Section,
    data: &ScheduleData,
    color_mode: ColorMode,
    empty_fill: Option<&str>,
) -> String {
    // Grid font sizes are global `#let`s from the preamble (`fonts::typst_lets`).
    let mut cfg = GridRenderConfig::full_page("", section.highlight_room);
    cfg.corner_label = section.corner_label.clone();
    cfg.highlight_panel_ids = section.highlight_panel_ids.clone();
    cfg.empty_fill = empty_fill.map(str::to_string);
    let layout = GridLayout::compute(
        &section.grid_panels,
        data,
        section.window_start.as_deref(),
        section.window_end.as_deref(),
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
        banner::page_header_running_split(
            brand,
            logo,
            &format!("[{}]", escape_literal(text)),
            &right,
        )
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

/// A labelled slice of panels with an optional visible time window.
///
/// `(label, panels, window_start, window_end)` where the window fields are
/// ISO 8601 datetime strings used to clamp the grid to the section boundary.
type TimedSection<'a> = (String, Vec<&'a Panel>, Option<String>, Option<String>);

/// Flatten `by_day` into time-labeled panel slices for `time`.
fn time_sections<'a>(
    time: &TimeSplit,
    by_day: &[(String, Vec<&'a Panel>)],
    all_dates: &[&str],
    timeline: &[TimelineEntry],
    schedule_start: Option<&str>,
    schedule_end: Option<&str>,
    tz: &str,
) -> Vec<TimedSection<'a>> {
    match time {
        TimeSplit::Day => by_day
            .iter()
            .map(|(date, panels)| (make_day_label(date, all_dates), panels.clone(), None, None))
            .collect(),
        TimeSplit::HalfDay => by_day
            .iter()
            .flat_map(|(date, panels)| {
                let day_label = make_day_label(date, all_dates);
                split_halves(&day_label, date, panels, tz)
            })
            .collect(),
        TimeSplit::Timeline => split_on_timeline(by_day, all_dates, timeline, tz),
        TimeSplit::Custom(ct) => {
            if let (Some(start), Some(end)) = (schedule_start, schedule_end) {
                split_on_custom_timeline(by_day, start, end, ct, tz)
            } else {
                // Fallback to day split if no schedule range available
                by_day
                    .iter()
                    .map(|(date, panels)| {
                        (make_day_label(date, all_dates), panels.clone(), None, None)
                    })
                    .collect()
            }
        }
    }
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
fn split_on_timeline<'a>(
    by_day: &[(String, Vec<&'a Panel>)],
    all_dates: &[&str],
    timeline: &[TimelineEntry],
    tz: &str,
) -> Vec<TimedSection<'a>> {
    // Build a sorted list of (date, start_time_str, label) from timeline entries
    // that have a start time. Entries without a time are skipped. The ISO start
    // is derived from the entry's epoch in the schedule timezone.
    let tl_starts: Vec<(String, &str)> = timeline
        .iter()
        .filter_map(|t| timeline_start_iso(t, tz).map(|s| (s, t.name.as_str())))
        .collect();
    let mut tl_boundaries: Vec<(&str, &str, &str)> = tl_starts
        .iter()
        .filter_map(|(start, name)| {
            let date = start.get(..10)?;
            Some((date, start.as_str(), *name))
        })
        .collect();
    // Sort by start_time so boundaries are in chronological order.
    tl_boundaries.sort_by_key(|(_, start, _)| *start);

    let mut out: Vec<TimedSection<'a>> = vec![];

    for (date, panels) in by_day {
        let day_label = make_day_label(date, all_dates);
        // Collect the timeline boundaries that fall on this day.
        let day_boundaries: Vec<(&str, &str)> = tl_boundaries
            .iter()
            .filter(|(d, _, _)| *d == date.as_str())
            .map(|(_, start, name)| (*start, *name))
            .collect();

        if day_boundaries.is_empty() {
            // No timeline entries for this day — fall back to a single day section.
            if !panels.is_empty() {
                out.push((day_label, panels.clone(), None, None));
            }
            continue;
        }

        // Assign each panel to the latest boundary whose start_time ≤ panel start_time.
        // Panels before the first boundary fall into a catch-all keyed by the day label.
        let section_label_for = |panel: &&'a Panel| -> String {
            let panel_start = match panel_start_iso(panel, tz) {
                Some(s) => s,
                None => return day_label.clone(),
            };
            // Walk boundaries in reverse to find the last one that starts ≤ panel.
            day_boundaries
                .iter()
                .rev()
                .find(|(bstart, _)| *bstart <= panel_start.as_str())
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| day_label.clone())
        };

        // Group panels preserving the boundary order: day-label bucket first,
        // then each boundary in chronological order.
        // section_keys[i] -> (key, win_start, win_end)
        // The day-label bucket's window is [None, first_boundary).
        // Each timeline-boundary bucket's window is [boundary_start, next_boundary_start).
        // The last bucket has no upper bound.
        let mut section_keys: Vec<(String, Option<String>, Option<String>)> = vec![(
            day_label.clone(),
            None,
            Some(day_boundaries[0].0.to_string()),
        )];
        for (i, (bstart, name)) in day_boundaries.iter().enumerate() {
            let key = name.to_string();
            let win_end = day_boundaries.get(i + 1).map(|(next, _)| next.to_string());
            if !section_keys.iter().any(|(k, _, _)| k == &key) {
                section_keys.push((key, Some(bstart.to_string()), win_end));
            }
        }

        let mut buckets: std::collections::HashMap<String, Vec<&'a Panel>> =
            std::collections::HashMap::new();
        for panel in panels {
            buckets
                .entry(section_label_for(panel))
                .or_default()
                .push(panel);
        }

        for (key, win_start, win_end) in section_keys {
            if let Some(bucket) = buckets.remove(&key) {
                if !bucket.is_empty() {
                    out.push((key, bucket, win_start, win_end));
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
    by_day: &[(String, Vec<&'a Panel>)],
    schedule_start: &str,
    schedule_end: &str,
    custom_timeline: &crate::config::CustomTimeline,
    tz: &str,
) -> Vec<TimedSection<'a>> {
    use crate::config::CustomTimeSlot;

    // Slots are already fully expanded with ISO 8601 times (done in `parse_time_split`).
    // Build a globally sorted list of (start_iso, explicit_end_iso_or_empty, label).
    let mut all_slots: Vec<(String, String, &str)> = custom_timeline
        .slots
        .iter()
        .filter_map(|s: &CustomTimeSlot| {
            let dt = crate::time_fmt::parse_loose_datetime(&s.time, schedule_start, schedule_end)?;
            let start_iso = dt.format("%Y-%m-%dT%H:%M:%S").to_string();
            let end_iso = s
                .end_time
                .as_deref()
                .and_then(|e| {
                    crate::time_fmt::parse_loose_datetime(e, schedule_start, schedule_end)
                })
                .map(|e| e.format("%Y-%m-%dT%H:%M:%S").to_string())
                .unwrap_or_default();
            Some((start_iso, end_iso, s.label.as_str()))
        })
        .collect();
    all_slots.sort_by(|a, b| a.0.cmp(&b.0));

    // Resolve effective end for each slot globally:
    // - explicit end_time wins if set
    // - otherwise the next slot's start (across any day boundary)
    // - the last slot has no upper bound
    let slot_windows: Vec<(&str, Option<&str>, &str)> = all_slots
        .iter()
        .enumerate()
        .map(|(i, (start, end, label))| {
            let effective_end = if !end.is_empty() {
                Some(end.as_str())
            } else if let Some((next_start, _, _)) = all_slots.get(i + 1) {
                Some(next_start.as_str())
            } else {
                None // last slot: no upper bound
            };
            (start.as_str(), effective_end, *label)
        })
        .collect();

    if slot_windows.is_empty() {
        return vec![];
    }

    // Determine whether panels before the first slot should be collected.
    // The first slot is "windowed" if it has an explicit end_time (the raw end
    // string in all_slots[0].1 is non-empty). Windowed first slots imply the
    // timeline covers only specific windows; anything before is intentionally
    // excluded. Open-ended first slots imply the timeline covers the whole
    // schedule; panels before it get a "Before <label>" catch-all section.
    let first_slot_is_windowed = !all_slots[0].1.is_empty();
    let before_label: Option<String> = if first_slot_is_windowed {
        None
    } else {
        let first_label = all_slots[0].2;
        if first_label.is_empty() {
            None
        } else {
            Some(format!("Before {}", first_label))
        }
    };
    let first_slot_start = all_slots[0].0.as_str();

    // Assign a panel to the slot whose window it falls within, or to the
    // before-section if the panel precedes the first slot.
    let section_label_for = |panel: &&'a Panel| -> Option<String> {
        let panel_start = panel_start_iso(panel, tz)?;
        let panel_start = panel_start.as_str();
        if panel_start < first_slot_start {
            return before_label.clone();
        }
        // Find the latest slot whose start ≤ panel_start and end bound not exceeded.
        slot_windows
            .iter()
            .rev()
            .find(|(slot_start, slot_end, _)| {
                *slot_start <= panel_start && slot_end.is_none_or(|e| panel_start < e)
            })
            .map(|(_, _, label)| label.to_string())
    };

    // Group all panels across all days into labelled buckets.
    let mut buckets: std::collections::HashMap<String, Vec<&'a Panel>> =
        std::collections::HashMap::new();
    for (_date, panels) in by_day {
        for panel in panels {
            if let Some(label) = section_label_for(panel) {
                buckets.entry(label).or_default().push(panel);
            }
            // Panels outside all slot windows are silently excluded
        }
    }

    let mut out: Vec<TimedSection<'a>> = vec![];

    // Prepend the "Before <label>" section if there are panels before the first slot.
    // Its window is [None, first_slot_start) — no lower bound, ends at first slot.
    if let Some(ref bl) = before_label {
        if let Some(bucket) = buckets.remove(bl.as_str()) {
            if !bucket.is_empty() {
                out.push((bl.clone(), bucket, None, Some(first_slot_start.to_string())));
            }
        }
    }

    // Output sections in slot order (skip empty-label sentinels, skip empty sections).
    // Carry the resolved (win_start, win_end) from slot_windows.
    for (win_start, win_end, label) in &slot_windows {
        if label.is_empty() {
            continue;
        }
        if let Some(bucket) = buckets.remove(*label) {
            if !bucket.is_empty() {
                out.push((
                    label.to_string(),
                    bucket,
                    Some(win_start.to_string()),
                    win_end.map(str::to_string),
                ));
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
fn spanning_into<'a>(all_panels: &[&'a Panel], window_start: &str, tz: &str) -> Vec<&'a Panel> {
    all_panels
        .iter()
        .copied()
        .filter(|p| {
            let start = panel_start_iso(p, tz).unwrap_or_default();
            let end = panel_end_iso(p, tz).unwrap_or_default();
            // Panel started before the window and ends after it begins.
            start.as_str() < window_start && end.as_str() > window_start
        })
        .collect()
}

/// Build the document's sections for the configured split.
fn build_sections<'a>(
    config: &LayoutConfig,
    data: &ScheduleData,
    panels: &[&'a Panel],
) -> Vec<Section<'a>> {
    let tz = data.meta.timezone.as_str();
    // Schedule window as local-wall-clock ISO, used for loose-time resolution.
    let meta_start = meta_start_iso(&data.meta);
    let meta_end = meta_end_iso(&data.meta);
    let all_date_strs: Vec<String> = unique_days(panels, tz);
    let all_dates: Vec<&str> = all_date_strs.iter().map(String::as_str).collect();
    let by_day = group_by_day(panels, tz);

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
        }],

        (None, Some(time)) => time_sections(
            &time,
            &by_day,
            &all_dates,
            &data.timeline,
            meta_start.as_deref(),
            meta_end.as_deref(),
            tz,
        )
        .into_iter()
        .map(|(label, time_panels, win_start, win_end)| {
            let grid_panels = if let Some(ref ws) = win_start {
                let mut gp = spanning_into(panels, ws, tz);
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
                corner_label: label,
                window_start: win_start,
                window_end: win_end,
            }
        })
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
                    corner_label: name,
                    window_start: None,
                    window_end: None,
                })
            })
            .collect(),

        (Some(SectionSplit::Room), Some(time)) => data
            .sorted_rooms()
            .iter()
            .flat_map(|room| {
                let name = room_name(room);
                time_sections(
                    &time,
                    &by_day,
                    &all_dates,
                    &data.timeline,
                    meta_start.as_deref(),
                    meta_end.as_deref(),
                    tz,
                )
                .into_iter()
                .filter_map(move |(time_label, time_panels, win_start, win_end)| {
                    let room_panels: Vec<&Panel> = time_panels
                        .iter()
                        .copied()
                        .filter(|p| p.room_ids.contains(&room.uid))
                        .collect();
                    if room_panels.is_empty() {
                        return None;
                    }
                    // grid_panels: time section panels + spanning ones for this room.
                    let grid_panels = if let Some(ref ws) = win_start {
                        let mut gp: Vec<&Panel> = spanning_into(panels, ws, tz)
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
                        corner_label: time_label,
                        window_start: win_start,
                        window_end: win_end,
                    })
                })
                .collect::<Vec<_>>()
            })
            .collect(),

        (Some(SectionSplit::Presenter), None) => data
            .presenters
            .iter()
            .filter(|p| postcard_rank_eligible(&p.rank))
            .filter_map(|presenter| {
                let his: Vec<&Panel> = panels
                    .iter()
                    .copied()
                    .filter(|p| p.presenters.iter().any(|n| n == &presenter.name))
                    .collect();
                if his.is_empty() {
                    return None;
                }
                Some(Section {
                    content_panels: his.clone(),
                    grid_panels: his,
                    highlight_room: None,
                    highlight_panel_ids: None,
                    left_label: String::new(),
                    right_label: presenter.name.clone(),
                    corner_label: presenter.name.clone(),
                    window_start: None,
                    window_end: None,
                })
            })
            .collect(),

        (Some(SectionSplit::Presenter), Some(time)) => data
            .presenters
            .iter()
            .filter(|p| postcard_rank_eligible(&p.rank))
            .flat_map(|presenter| {
                time_sections(
                    &time,
                    &by_day,
                    &all_dates,
                    &data.timeline,
                    meta_start.as_deref(),
                    meta_end.as_deref(),
                    tz,
                )
                .into_iter()
                .filter_map(move |(time_label, time_panels, win_start, win_end)| {
                    let his: Vec<&Panel> = time_panels
                        .iter()
                        .copied()
                        .filter(|p| p.presenters.iter().any(|n| n == &presenter.name))
                        .collect();
                    if his.is_empty() {
                        return None;
                    }
                    let ids: HashSet<String> = his.iter().map(|p| p.id.clone()).collect();
                    // grid_panels: time section panels + spanning ones for this presenter.
                    let grid_panels = if let Some(ref ws) = win_start {
                        let mut gp: Vec<&Panel> = spanning_into(panels, ws, tz)
                            .into_iter()
                            .filter(|p| p.presenters.iter().any(|n| n == &presenter.name))
                            .collect();
                        gp.extend_from_slice(&time_panels);
                        gp
                    } else {
                        time_panels.clone()
                    };
                    Some(Section {
                        content_panels: his,
                        grid_panels,
                        highlight_room: None,
                        highlight_panel_ids: Some(ids),
                        left_label: presenter.name.clone(),
                        right_label: time_label.clone(),
                        corner_label: time_label,
                        window_start: win_start,
                        window_end: win_end,
                    })
                })
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

/// Unique calendar days (YYYY-MM-DD) present in the panel set, first-seen order.
fn unique_days(panels: &[&Panel], tz: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut days = vec![];
    for p in panels {
        if let Some(d) = panel_start_iso(p, tz).and_then(|s| s.get(..10).map(str::to_string)) {
            if seen.insert(d.clone()) {
                days.push(d);
            }
        }
    }
    days
}

/// Group scheduled panels by calendar day, preserving first-seen order.
fn group_by_day<'a>(panels: &[&'a Panel], tz: &str) -> Vec<(String, Vec<&'a Panel>)> {
    let mut by_day: Vec<(String, Vec<&'a Panel>)> = vec![];
    for panel in panels {
        if let Some(start) = panel_start_iso(panel, tz) {
            let date = start.get(..10).unwrap_or("unknown").to_string();
            if let Some(entry) = by_day.iter_mut().find(|(d, _)| d == &date) {
                entry.1.push(panel);
            } else {
                by_day.push((date, vec![panel]));
            }
        }
    }
    by_day
}

/// Split a day's panels into AM and PM halves, dropping empty halves.
///
/// `date` is the `YYYY-MM-DD` string for the day being split; it is used to
/// build the noon window boundary (`YYYY-MM-DDT12:00`) so that panels which
/// span the AM/PM boundary are visually clamped in each half's grid.
fn split_halves<'a>(
    day_label: &str,
    date: &str,
    panels: &[&'a Panel],
    tz: &str,
) -> Vec<TimedSection<'a>> {
    let hour_of = |p: &&'a Panel| -> Option<u32> {
        panel_start_iso(p, tz)
            .as_deref()
            .and_then(|s| s.get(11..13).and_then(|h| h.parse::<u32>().ok()))
    };

    let noon = format!("{}T12:00", date);

    let am: Vec<&'a Panel> = panels
        .iter()
        .copied()
        .filter(|p| hour_of(p).map(|h| h < 12).unwrap_or(false))
        .collect();
    let pm: Vec<&'a Panel> = panels
        .iter()
        .copied()
        .filter(|p| hour_of(p).map(|h| h >= 12).unwrap_or(false))
        .collect();

    let mut out = vec![];
    if !am.is_empty() {
        // AM window: no lower bound (starts at first event), ends at noon.
        out.push((format!("{} AM", day_label), am, None, Some(noon.clone())));
    }
    if !pm.is_empty() {
        // PM window: starts at noon, no upper bound.
        out.push((format!("{} PM", day_label), pm, Some(noon), None));
    }
    out
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
    use crate::model::{Meta, Panel, Presenter, Room, ScheduleData};

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
            presenters: if presenter.is_empty() {
                vec![]
            } else {
                vec![presenter.to_string()]
            },
            ..Panel::default()
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
            ..Presenter::default()
        }];
        d.panels = vec![
            panel("P1", "26T09", 1, "Ada"), // Fri AM, room A, Ada
            panel("P2", "26T14", 2, ""),    // Fri PM, room B
            panel("P3", "27T10", 1, "Ada"), // Sat AM, room A, Ada
        ];
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
