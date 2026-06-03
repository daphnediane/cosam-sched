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
//! [`ContentMode`] (what to draw) and [`SplitMode`] (how to break it into
//! sections):
//!
//! - [`ContentMode::Both`]: grid on the left half of each section, descriptions
//!   flowing through the remaining columns.
//! - [`ContentMode::GridOnly`]: full-width schedule grid per section.
//! - [`ContentMode::DescriptionOnly`]: multi-column descriptions; page breaks
//!   between sections (`None` split = one continuous flow).
//! - [`ContentMode::PanelList`]: compact name + time + room list (former guest
//!   postcards).
//!
//! Grid-bearing content collapses `Room`/`Presenter` splits to their per-day
//! form (a grid spans a single day). `Room`/`RoomDay` highlight the section's
//! room column; `Presenter`/`PresenterDay` highlight the guest's own cells in
//! the day grid. [`LayoutConfig::double_sided`] pads each section onto an odd
//! page; [`LayoutConfig::header_text`] and [`FooterMode`] drive the banners.

use std::collections::HashSet;

use crate::blocks::banner;
use crate::blocks::grid::{render_schedule_grid, GridRenderConfig};
use crate::blocks::panels::{render_panel_list, render_time_grouped_panels};
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::{ContentMode, FooterMode, GridLayout, LayoutConfig, PanelFilter, SplitMode};
use crate::model::{Panel, ScheduleData};
use crate::typst_gen::{make_day_label, preamble};

/// One renderable section of the document.
struct Section<'a> {
    /// Panels shown in the descriptions/list.
    content_panels: Vec<&'a Panel>,
    /// Panels the grid is built from (the full day for room/presenter sections).
    grid_panels: Vec<&'a Panel>,
    /// Highlight this room's grid column (Room/RoomDay).
    highlight_room: Option<i64>,
    /// Highlight these event cells by panel id (Presenter/PresenterDay).
    highlight_panel_ids: Option<HashSet<String>>,
    /// Running-header left label (2-D entity); empty otherwise.
    left_label: String,
    /// Running-header right label (day, or the 1-D section label); empty for None.
    right_label: String,
    /// Grid corner-cell text.
    corner_label: String,
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

    let content = config.content;
    let split = content.effective_split();
    let font_pt = config.effective_font_pt();

    let mut doc = preamble(config, brand);

    // The header bar is always present (fixed top margin); widen the bottom only
    // when a footer is shown (the preamble's bottom margin is tuned for
    // edge-to-edge grids).
    let bottom = if matches!(config.footer, FooterMode::None) {
        "0.125in"
    } else {
        "0.5in"
    };
    doc.push_str(&format!(
        "#set page(margin: (top: 0.625in, bottom: {bottom}, left: 0.125in, right: 0.125in), \
         footer-descent: 0.15in)\n",
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
    doc.push_str(&build_header(brand, config, split));

    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            if config.double_sided {
                doc.push_str("#pagebreak(to: \"odd\")\n\n");
            } else {
                doc.push_str("#pagebreak()\n\n");
            }
        }

        if split != SplitMode::None {
            doc.push_str(&section_marker(&section.left_label, &section.right_label));
        }

        match content {
            ContentMode::GridOnly(_) => {
                doc.push_str(&render_grid(section, data, config, color_mode));
            }
            ContentMode::Both(_) => {
                let total_cols =
                    config.effective_columns(config.paper.flyer_columns(config.orientation));
                let grid_cols = total_cols.div_ceil(2);
                let grid_pct = grid_cols as f64 / total_cols as f64 * 100.0;

                doc.push_str(&format!(
                    "#place(top + left, box(width: {:.2}%)[\n",
                    grid_pct
                ));
                doc.push_str(&render_grid(section, data, config, color_mode));
                doc.push_str("])\n");

                doc.push_str(&format!("#columns({}, gutter: 0.2in)[\n", total_cols));
                for _ in 0..grid_cols {
                    doc.push_str("#colbreak()\n");
                }
                doc.push_str(&render_time_grouped_panels(
                    data,
                    color_mode,
                    &section.content_panels,
                    font_pt,
                ));
                doc.push_str("]\n");
            }
            ContentMode::DescriptionOnly(_) => {
                let total_cols = config
                    .effective_columns(config.paper.description_columns(config.orientation));
                doc.push_str(&format!("#columns({}, gutter: 0.2in)[\n", total_cols));
                doc.push_str(&render_time_grouped_panels(
                    data,
                    color_mode,
                    &section.content_panels,
                    font_pt,
                ));
                doc.push_str("]\n");
            }
            ContentMode::PanelList(_) => {
                let total_cols = config
                    .effective_columns(config.paper.description_columns(config.orientation));
                doc.push_str(&format!("#columns({}, gutter: 0.2in)[\n", total_cols));
                doc.push_str(&render_panel_list(
                    data,
                    color_mode,
                    &section.content_panels,
                    font_pt,
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
    config: &LayoutConfig,
    color_mode: ColorMode,
) -> String {
    let mut cfg = GridRenderConfig::full_page("", section.highlight_room)
        .with_base_font(config.grid_font_value());
    cfg.corner_label = section.corner_label.clone();
    cfg.highlight_panel_ids = section.highlight_panel_ids.clone();
    let layout = GridLayout::compute(&section.grid_panels, data);
    render_schedule_grid(&layout, data, color_mode, &cfg)
}

/// Build the page header directive for the chosen split.
fn build_header(brand: &BrandConfig, config: &LayoutConfig, split: SplitMode) -> String {
    if split == SplitMode::None {
        // Static header; header_text on the right.
        return banner::page_header(brand, None, config.header_text.as_deref());
    }
    let right = running_field("right");
    if split.is_two_dim() {
        // Entity left, day right; header_text omitted (both slots taken).
        banner::page_header_running_split(brand, &running_field("left"), &right)
    } else if let Some(text) = config.header_text.as_deref() {
        // header_text literal on the left, running section label on the right.
        banner::page_header_running_split(brand, &format!("[{}]", escape_literal(text)), &right)
    } else {
        // Logo left, running section label right.
        banner::page_header_running(brand, &right)
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

/// Build the document's sections for the configured split.
fn build_sections<'a>(
    config: &LayoutConfig,
    data: &ScheduleData,
    panels: &[&'a Panel],
) -> Vec<Section<'a>> {
    let all_date_strs: Vec<String> = unique_days(panels);
    let all_dates: Vec<&str> = all_date_strs.iter().map(String::as_str).collect();
    let by_day = group_by_day(panels);

    match config.content.effective_split() {
        SplitMode::None => vec![Section {
            content_panels: panels.to_vec(),
            grid_panels: panels.to_vec(),
            highlight_room: None,
            highlight_panel_ids: None,
            left_label: String::new(),
            right_label: String::new(),
            corner_label: String::new(),
        }],

        SplitMode::Day => by_day
            .iter()
            .map(|(date, day_panels)| {
                let label = make_day_label(date, &all_dates);
                Section {
                    content_panels: day_panels.clone(),
                    grid_panels: day_panels.clone(),
                    highlight_room: None,
                    highlight_panel_ids: None,
                    left_label: String::new(),
                    right_label: label.clone(),
                    corner_label: label,
                }
            })
            .collect(),

        SplitMode::HalfDay => by_day
            .iter()
            .flat_map(|(date, day_panels)| {
                let day_label = make_day_label(date, &all_dates);
                split_halves(&day_label, day_panels)
                    .into_iter()
                    .map(|(label, half_panels)| Section {
                        content_panels: half_panels.clone(),
                        grid_panels: half_panels,
                        highlight_room: None,
                        highlight_panel_ids: None,
                        left_label: String::new(),
                        right_label: label.clone(),
                        corner_label: label,
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),

        SplitMode::Room => data
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
                })
            })
            .collect(),

        SplitMode::RoomDay => data
            .sorted_rooms()
            .iter()
            .flat_map(|room| {
                let name = room_name(room);
                by_day
                    .iter()
                    .filter_map(|(date, day_panels)| {
                        let room_panels: Vec<&Panel> = day_panels
                            .iter()
                            .copied()
                            .filter(|p| p.room_ids.contains(&room.uid))
                            .collect();
                        if room_panels.is_empty() {
                            return None;
                        }
                        let day_label = make_day_label(date, &all_dates);
                        Some(Section {
                            content_panels: room_panels,
                            grid_panels: day_panels.clone(),
                            highlight_room: Some(room.uid),
                            highlight_panel_ids: None,
                            left_label: name.clone(),
                            right_label: day_label.clone(),
                            corner_label: day_label,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),

        SplitMode::Presenter => data
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
                })
            })
            .collect(),

        SplitMode::PresenterDay => data
            .presenters
            .iter()
            .filter(|p| postcard_rank_eligible(&p.rank))
            .flat_map(|presenter| {
                by_day
                    .iter()
                    .filter_map(|(date, day_panels)| {
                        let his: Vec<&Panel> = day_panels
                            .iter()
                            .copied()
                            .filter(|p| p.presenters.iter().any(|n| n == &presenter.name))
                            .collect();
                        if his.is_empty() {
                            return None;
                        }
                        let ids: HashSet<String> = his.iter().map(|p| p.id.clone()).collect();
                        let day_label = make_day_label(date, &all_dates);
                        Some(Section {
                            content_panels: his,
                            grid_panels: day_panels.clone(),
                            highlight_room: None,
                            highlight_panel_ids: Some(ids),
                            left_label: presenter.name.clone(),
                            right_label: day_label.clone(),
                            corner_label: day_label,
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
fn unique_days(panels: &[&Panel]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut days = vec![];
    for p in panels {
        if let Some(d) = p.start_time.as_deref().and_then(|s| s.get(..10)) {
            if seen.insert(d.to_string()) {
                days.push(d.to_string());
            }
        }
    }
    days
}

/// Group scheduled panels by calendar day, preserving first-seen order.
fn group_by_day<'a>(panels: &[&'a Panel]) -> Vec<(String, Vec<&'a Panel>)> {
    let mut by_day: Vec<(String, Vec<&'a Panel>)> = vec![];
    for panel in panels {
        if let Some(start) = &panel.start_time {
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
fn split_halves<'a>(day_label: &str, panels: &[&'a Panel]) -> Vec<(String, Vec<&'a Panel>)> {
    let hour_of = |p: &&'a Panel| -> Option<u32> {
        p.start_time
            .as_ref()
            .and_then(|s| s.get(11..13))
            .and_then(|h| h.parse::<u32>().ok())
    };

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
        out.push((format!("{} AM", day_label), am));
    }
    if !pm.is_empty() {
        out.push((format!("{} PM", day_label), pm));
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
    use crate::grid::{LayoutConfig, PaperSize};
    use crate::model::{Meta, Panel, Presenter, Room, ScheduleData};
    use std::collections::HashMap;

    fn empty_schedule() -> ScheduleData {
        ScheduleData {
            meta: Meta {
                title: "T".into(),
                version: 0,
                variant: String::new(),
                generator: String::new(),
                generated: String::new(),
                modified: String::new(),
                start_time: None,
                end_time: None,
            },
            panels: vec![],
            rooms: vec![],
            panel_types: HashMap::new(),
            timeline: vec![],
            presenters: vec![],
        }
    }

    fn panel(id: &str, day_hour: &str, room: i64, presenter: &str) -> Panel {
        Panel {
            id: id.into(),
            base_id: id.into(),
            name: format!("Panel {id}"),
            room_ids: vec![room],
            start_time: Some(format!("2026-06-{day_hour}:00")),
            end_time: Some(format!("2026-06-{day_hour}:00")),
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
                hotel_room: String::new(),
                sort_key: 0,
            },
            Room {
                uid: 2,
                short_name: "B".into(),
                long_name: "Salon B".into(),
                hotel_room: String::new(),
                sort_key: 1,
            },
        ];
        d.presenters = vec![Presenter {
            uid: "Ada".into(),
            name: "Ada".into(),
            short_name: None,
            rank: "guest".into(),
        }];
        d.panels = vec![
            panel("P1", "26T09", 1, "Ada"), // Fri AM, room A, Ada
            panel("P2", "26T14", 2, ""),     // Fri PM, room B
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
        let c = cfg(ContentMode::DescriptionOnly(SplitMode::Day));
        let panels = filter_panels(&d, d.scheduled_panels(), PanelFilter::All);
        let secs = build_sections(&c, &d, &panels);
        assert_eq!(secs.len(), 2); // two days
    }

    #[test]
    fn test_sections_room_day_uses_full_day_grid() {
        let d = two_day_schedule();
        let c = cfg(ContentMode::Both(SplitMode::RoomDay));
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
    fn test_sections_presenter_collapses_to_day_with_grid() {
        let d = two_day_schedule();
        // Presenter split + grid content collapses to PresenterDay.
        let c = cfg(ContentMode::Both(SplitMode::Presenter));
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
        let c = cfg(ContentMode::PanelList(SplitMode::Presenter));
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
            PaperSize::Letter.flyer_columns(crate::grid::Orientation::Landscape),
            4
        );
        assert_eq!(
            PaperSize::Legal.flyer_columns(crate::grid::Orientation::Landscape),
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
