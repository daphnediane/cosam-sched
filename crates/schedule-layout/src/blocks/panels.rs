/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared panel description block rendering for description and workshop listings.
//!
//! ## Primary entry point
//!
//! - [`render_time_grouped_panels`] — full column-document rendering with sticky
//!   level-2 headings and Typst-state-based repeated headers at column breaks.
//!   Used by the `descriptions`, `workshops_listing`, `flyer`, and `room_signs`
//!   formats.

use std::collections::{HashMap, HashSet};

use schedule_core::value::uniq_id::PanelUniqId;

use crate::color::{ColorMode, PanelColor};
use crate::model::{Panel, ScheduleData};
use crate::time_fmt;
use crate::typst_gen::escape_typst;

/// Return type of [`build_time_groups`]: base-id lookup + ordered time groups.
type TimeGroups<'a> = (
    HashMap<&'a str, Vec<&'a Panel>>,
    Vec<(String, Vec<&'a Panel>)>,
);

/// Visual style for description panel blocks.
pub(crate) struct PanelStyle {
    /// Render as a bordered card (colored left spine + light border) instead of
    /// the original full-height left accent bar.
    pub card: bool,
    /// Card background as a Typst color expression (used when [`card`](Self::card)).
    pub card_fill: String,
    /// When `Some`, emit `below: <expr>` on each panel block to set the
    /// inter-panel gap; `None` keeps Typst's default block spacing.
    pub gap: Option<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render time-grouped panel blocks for use inside a `#columns()` section.
///
/// Handles both single-day (descriptions) and multi-day (workshops) cases.
/// Panels are grouped by day first, then by time slot. Day headings are
/// automatically inserted when the date changes.
///
/// For slots with more than one panel, each heading and panel block receives a
/// Typst label. Before each non-first panel, a `#context` block queries the
/// previous element's label to detect column/page breaks and inserts a
/// "(continued)" heading when needed. Using labels + `query` (read-only) instead
/// of `state.update()` (read-write) avoids the layout-convergence feedback loop
/// that caused "layout did not converge" warnings.
pub(crate) fn render_time_grouped_panels<'a>(
    data: &'a ScheduleData,
    color_mode: ColorMode,
    panels: &[&'a Panel],
    style: &PanelStyle,
) -> String {
    let (by_base, time_groups) = build_time_groups(panels);

    let mut out = String::new();
    let mut slot_counter = 0u32;

    // Collect unique days for smart label generation
    let all_days: Vec<&str> = panels
        .iter()
        .filter_map(|p| p.start_time.as_deref().and_then(|s| s.get(..10)))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Sticky show rule so level-2 headings never orphan at a column bottom.
    out.push_str("#show heading.where(level: 2): set block(sticky: true)\n\n");

    for (time_key, group) in &time_groups {
        // Extract day from time_key (YYYY-MM-DDTHH:MM)
        let day_str = time_key.get(..10).unwrap_or("");
        let day_label = crate::typst_gen::make_day_label(day_str, &all_days);

        // Extract time portion for the slot label
        let slot_label = time_fmt::format_time(time_key);
        if slot_label.is_empty() {
            for panel in group {
                out.push_str(&panel_block(
                    data, color_mode, panel, day_str, &by_base, style, None,
                ));
            }
            continue;
        }

        let full_slot_label = format!("{} {}", day_label, slot_label);

        if group.len() == 1 {
            out.push_str(&format!("== {}\n\n", escape_typst(&full_slot_label)));
            out.push_str(&panel_block(
                data, color_mode, group[0], day_str, &by_base, style, None,
            ));
        } else {
            // Label scheme for this slot (n = slot_counter):
            //   heading  → <_slot_n_0>  (sticky, moves with first panel)
            //   panel i  → <_slot_n_{i+1}>
            //
            // Before panel i (i > 0) we query <_slot_n_{i}> (the previous element)
            // to detect column/page breaks. Because we never call `state.update()`
            // inside a `context` block, layout converges without oscillation.
            //
            // The query is scoped with `.before(here())` and takes the *last*
            // match so that callers which invoke this builder more than once per
            // document (e.g. the flyer, one call per day) — and therefore reuse
            // the `_slot_n` tags — resolve to the current section's element
            // rather than an identically-tagged one from an earlier section.
            let slot_tag = format!("_slot_{}", slot_counter);
            slot_counter += 1;

            out.push_str(&format!(
                "== {lbl} <{tag}_0>\n\n",
                lbl = escape_typst(&full_slot_label),
                tag = slot_tag,
            ));

            let cont_label = format!(
                "#text(size: {secondary_size}, style: \"italic\")[(continued)]",
                secondary_size = SECONDARY_SIZE,
            );

            for (i, panel) in group.iter().enumerate() {
                if i > 0 {
                    // Label of the immediately preceding element (heading when i==1,
                    // previous panel block when i>1).
                    let prev_tag = format!("{slot_tag}_{i}");
                    out.push_str(&format!(
                        "#context {{\n  \
                           let _hits = query(selector(label(\"{prev_tag}\")).before(here()))\n  \
                           if _hits.len() > 0 {{\n    \
                             let _hp = _hits.last().location().position()\n    \
                             let _p = here().position()\n    \
                             if _p.page != _hp.page or calc.abs((_p.x - _hp.x).pt()) > {threshold} [\n      \
                               == {lbl} {cont}\n    \
                             ]\n  \
                           }}\n\
                         }}\n\n",
                        prev_tag = prev_tag,
                        threshold = COLBREAK_THRESHOLD_PT,
                        lbl = escape_typst(&full_slot_label),
                        cont = cont_label,
                    ));
                }
                let panel_label = format!("{slot_tag}_{}", i + 1);
                out.push_str(&panel_block(
                    data,
                    color_mode,
                    panel,
                    day_str,
                    &by_base,
                    style,
                    Some(&panel_label),
                ));
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// panel-list layout constants
// ---------------------------------------------------------------------------

/// Vertical space *above* a panel-list day heading (em units).
/// Emitted as a Typst literal — not a bare number — so changes here
/// propagate to the generated source without scattering magic strings.
const PANEL_LIST_HEADING_ABOVE: &str = "0.8em";

/// Vertical space *below* a panel-list day heading (em units).
const PANEL_LIST_HEADING_BELOW: &str = "0.3em";

/// Width of the accent bar column. The cell is filled with the panel color, so
/// the column width *is* the bar width; the grid's `column-gutter` supplies the
/// breathing room to the time column.
const ACCENT_COL_WIDTH: &str = "3pt";

/// Vertical gap between consecutive panel-list rows.
const PANEL_LIST_ROW_BELOW: &str = "0.4em";

/// Width of the right-aligned hour column in the time sub-grid.
/// Large enough for two digits ("10") in the secondary font.
const TIME_HOUR_COL: &str = "1.4em";

/// Width of the left-aligned suffix column in the time sub-grid.
/// Large enough for ":30 PM" (the widest suffix) on a single line.
const TIME_SUFFIX_COL: &str = "3.4em";

// ---------------------------------------------------------------------------
// render_panel_list
// ---------------------------------------------------------------------------

/// Render a compact panel list (name + stacked time range + room) for a
/// section, inserting a small day heading whenever the date changes.
///
/// Used by the `PanelList` content mode (guest-postcard layout).
///
/// ## Grid layout (5 columns, up to 3 rows per panel)
///
/// ```text
/// col 0: accent bar (ACCENT_COL_WIDTH, filled rect or empty)
/// col 1: hour       (TIME_HOUR_COL,    right-aligned)
/// col 2: suffix     (TIME_SUFFIX_COL,  left-aligned)
/// col 3: name       (1fr,              left + top, rowspan 1–3)
/// col 4: room       (auto,             right + top, rowspan 1–3)
/// ```
///
/// Rows per panel:
/// - row 1: accent (rowspan) | start-hour | start-suffix | name (rowspan) | room (rowspan)
/// - row 2: (accent cont.)  | separator spanning cols 1–2 (center)
/// - row 3: (accent cont.)  | end-hour   | end-suffix
///
/// All cell contents are plain Typst values in code context — no `[…]`
/// wrapper is placed around the whole grid, so there is no markup/code
/// context confusion and no need for `#` prefixes inside cell bodies.
///
/// ## Day headings
///
/// `== Day ==` headings use `breakable: false` so they stay visually attached
/// to the rows below without ever forcing a column/page break.
pub(crate) fn render_panel_list<'a>(
    data: &'a ScheduleData,
    color_mode: ColorMode,
    panels: &[&'a Panel],
) -> String {
    // Stable chronological order.
    let mut ordered: Vec<&Panel> = panels.to_vec();
    ordered.sort_by(|a, b| {
        a.start_time
            .as_deref()
            .unwrap_or("")
            .cmp(b.start_time.as_deref().unwrap_or(""))
    });

    let all_days: Vec<&str> = ordered
        .iter()
        .filter_map(|p| p.start_time.as_deref().and_then(|s| s.get(..10)))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let mut out = String::new();

    // Day headings: breakable: false keeps them attached to the next row
    // without ever forcing a column/page break (unlike `sticky: true`).
    out.push_str(&format!(
        "#show heading.where(level: 2): it => \
         block(breakable: false, above: {above}, below: {below})[#it.body]\n\n",
        above = PANEL_LIST_HEADING_ABOVE,
        below = PANEL_LIST_HEADING_BELOW,
    ));

    let mut current_day = "";
    for panel in &ordered {
        let day_str = panel
            .start_time
            .as_deref()
            .and_then(|s| s.get(..10))
            .unwrap_or("");
        if day_str != current_day {
            current_day = day_str;
            let label = crate::typst_gen::make_day_label(day_str, &all_days);
            out.push_str(&format!("== {}\n\n", escape_typst(&label)));
        }

        let room = panel
            .room_ids
            .first()
            .and_then(|uid| data.rooms.iter().find(|r| r.uid == *uid))
            .map(|r| {
                if !r.short_name.is_empty() {
                    r.short_name.as_str()
                } else {
                    r.long_name.as_str()
                }
            })
            .unwrap_or("");

        let color_str = panel
            .panel_type
            .as_ref()
            .and_then(|pt| data.panel_types.get(pt.as_str()))
            .and_then(|pt| PanelColor::resolve(&pt.colors, color_mode))
            .map(|c| c.hex)
            .unwrap_or_default();

        // Split start and end times into (hour, suffix) for digit alignment.
        let (start_hour, start_suffix) = panel
            .start_time
            .as_deref()
            .map(time_fmt::format_time_split)
            .unwrap_or_default();
        let (end_hour, end_suffix) = panel
            .end_time
            .as_deref()
            .map(time_fmt::format_time_split)
            .unwrap_or_default();
        let has_end = !end_hour.is_empty();
        let row_count = if has_end { 3 } else { 1 };

        let sz = SECONDARY_SIZE;
        let name_esc = escape_typst(&panel.name);

        // ── Build cell fragments ──────────────────────────────────────────
        //
        // Typst bracket-parser rules governing all choices below:
        //   • `#block(…)[body]`    → `[body]` is MARKUP context.
        //   • `#grid(…)` in markup → the `(…)` args are CODE context.
        //   • `grid.cell(…)[body]` → `[body]` is MARKUP context.
        //   • In markup `[body]`, a bare `[` opens a NEW content block, so
        //     `text(size:sz)[val]` always breaks — the inner `]` closes the
        //     cell before the outer one can. This is true even when `val`
        //     has no brackets, because the `[` after `text(…)` is the issue.
        //   • `#text(size:sz)[val]` WITH the `#` prefix works: `#` switches
        //     to code mode for that call, so `[val]` is its content arg, and
        //     the parser correctly matches the brackets as a pair.
        //   • Direct grid args (`text(sz)[val]` NOT inside a cell `[body]`)
        //     are in code context — always fine.
        //
        // Rule: inside any `grid.cell(…)[BODY]`, use `#text(…)[val]`.
        //       As a direct grid arg (code context), use `text(…)[val]`.

        // col 0 – accent bar (rowspan). A filled `grid.cell` paints the panel
        // color across the cell's full height (all spanned rows + their inner
        // gutters), which is what we want. A `#rect(height: 100%)` would resolve
        // its height against the page/column, not the row, and stretch to the
        // page bottom — the same bug the description accent bars once had.
        let accent_cell = if color_str.is_empty() {
            format!("grid.cell(rowspan: {row_count})[],")
        } else {
            format!(
                "grid.cell(rowspan: {row_count}, fill: rgb(\"{color}\"))[],",
                color = color_str,
            )
        };

        // row 1 – start time.
        // Normal: direct code-context grid args — `text(sz)[val]` is fine.
        // Noon/Midnight: inside a cell body → must use `#text(sz)[val]`.
        let start_cells = if start_suffix.is_empty() {
            format!(
                "grid.cell(colspan: 2, align: center)[#text(size: {sz})[{hour}]],",
                sz = sz,
                hour = escape_typst(&start_hour),
            )
        } else {
            format!(
                "text(size: {sz})[{hour}], text(size: {sz})[{suffix}],",
                sz = sz,
                hour = escape_typst(&start_hour),
                suffix = escape_typst(&start_suffix),
            )
        };

        // col 3 – panel name (rowspan): plain markup text — no function call
        // with `[…]` content, so no bracket nesting issue.
        let name_cell = format!(
            "grid.cell(rowspan: {row_count}, align: left + top)[{name}],",
            name = name_esc,
        );

        // col 4 – room (rowspan): inside cell body → `#text(sz)[val]`.
        let room_cell = format!(
            "grid.cell(rowspan: {row_count}, align: right + top)[#text(size: {sz})[{room}]],",
            sz = sz,
            room = escape_typst(room),
        );

        // rows 2+3 – separator and end time (only when end time is present).
        let extra_rows = if has_end {
            // Separator: inside cell body → `#text`.
            let sep = format!(
                "grid.cell(colspan: 2, align: center)[#text(size: {sz})[|]],",
                sz = sz,
            );
            // End time: same rules as start time.
            let end_cells = if end_suffix.is_empty() {
                format!(
                    "grid.cell(colspan: 2, align: center)[#text(size: {sz})[{hour}]],",
                    sz = sz,
                    hour = escape_typst(&end_hour),
                )
            } else {
                format!(
                    "text(size: {sz})[{hour}], text(size: {sz})[{suffix}],",
                    sz = sz,
                    hour = escape_typst(&end_hour),
                    suffix = escape_typst(&end_suffix),
                )
            };
            format!("\n  {sep}\n  {end_cells}")
        } else {
            String::new()
        };

        // ── emit: #block(breakable: false, below: …)[#grid(…)] ──────────
        // `#block` keeps all rows of this panel together and provides the
        // correct inter-panel gap inside `#columns`. `#grid` sits inside
        // the block's markup `[body]` so it needs the `#` prefix.
        out.push_str(&format!(
            "#block(breakable: false, below: {row_below})[#grid(\
             columns: ({acol}, {hcol}, {scol}, 1fr, auto), \
             align: (left, right, left, left + top, right + top), \
             column-gutter: 2pt, row-gutter: 1pt, \
             {accent_cell}\n  \
             {start_cells}\n  \
             {name_cell}\n  \
             {room_cell}\
             {extra_rows})]\n",
            row_below = PANEL_LIST_ROW_BELOW,
            acol = ACCENT_COL_WIDTH,
            hcol = TIME_HOUR_COL,
            scol = TIME_SUFFIX_COL,
            accent_cell = accent_cell,
            start_cells = start_cells,
            name_cell = name_cell,
            room_cell = room_cell,
            extra_rows = extra_rows,
        ));
    }

    out
}

/// Horizontal shift (points) past which a panel is considered to have moved to a
/// new column, triggering a repeated "(continued)" slot heading.
const COLBREAK_THRESHOLD_PT: u32 = 50;

/// Font-size `#let` (from the preamble's `fonts::typst_lets`) used for secondary
/// text — credits, the right-hand room/time/cost stack, and "(continued)" tags.
const SECONDARY_SIZE: &str = "_desc-secondary-size";

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build the `by_base` lookup and ordered `time_groups` for a panel slice.
///
/// Panels are deduplicated by `id`; time groups maintain insertion order.
fn build_time_groups<'a>(panels: &[&'a Panel]) -> TimeGroups<'a> {
    let mut by_base: HashMap<&'a str, Vec<&'a Panel>> = HashMap::new();
    for p in panels.iter().copied() {
        by_base.entry(p.base_id.as_str()).or_default().push(p);
    }

    let mut time_groups: Vec<(String, Vec<&Panel>)> = vec![];
    let mut seen_ids: HashSet<&str> = HashSet::new();
    for panel in panels.iter().copied() {
        if !seen_ids.insert(panel.id.as_str()) {
            continue;
        }
        let key = panel
            .start_time
            .as_deref()
            .and_then(|s| s.get(..16))
            .unwrap_or("")
            .to_string();
        if let Some(group) = time_groups.iter_mut().find(|(k, _)| k == &key) {
            group.1.push(panel);
        } else {
            time_groups.push((key, vec![panel]));
        }
    }

    (by_base, time_groups)
}

/// Render a single panel as a Typst block.
///
/// `typst_label` is an optional Typst label name (without angle brackets) to
/// attach to the block element, used by the column-break detection in
/// [`render_time_grouped_panels`].
fn panel_block<'a>(
    data: &'a ScheduleData,
    color_mode: ColorMode,
    panel: &'a Panel,
    day_date: &str,
    by_base: &HashMap<&'a str, Vec<&'a Panel>>,
    style: &PanelStyle,
    typst_label: Option<&str>,
) -> String {
    let color_str = panel
        .panel_type
        .as_ref()
        .and_then(|pt| data.panel_types.get(pt.as_str()))
        .and_then(|pt| PanelColor::resolve(&pt.colors, color_mode))
        .map(|c| c.hex)
        .unwrap_or_default();

    let time_range =
        time_fmt::format_time_range(panel.start_time.as_deref(), panel.end_time.as_deref());

    let room_str = panel
        .room_ids
        .iter()
        .filter_map(|uid| data.rooms.iter().find(|r| r.uid == *uid))
        .map(|r| {
            if !r.long_name.is_empty() {
                r.long_name.as_str()
            } else {
                r.short_name.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    // Block style: either a bordered card (colored left spine + light border, so
    // header and body share one region) or the original full-height left accent
    // bar drawn as the block's left stroke. The inter-panel `below` gap is applied
    // only when `style.gap` is set (cards, or an explicit `panel_gap`).
    let style_attrs = if style.card {
        let spine = if color_str.is_empty() {
            "0.5pt + luma(80%)".to_string()
        } else {
            format!("2.5pt + rgb(\"{}\")", color_str)
        };
        format!(
            ", fill: {fill}, stroke: (left: {spine}, rest: 0.5pt + luma(80%)), \
             inset: (left: 8pt, rest: 6pt), radius: 2pt",
            fill = style.card_fill,
            spine = spine,
        )
    } else if color_str.is_empty() {
        String::new()
    } else {
        format!(
            ", stroke: (left: 3pt + rgb(\"{}\")), inset: (left: 6pt)",
            color_str
        )
    };
    let gap_attr = style
        .gap
        .as_deref()
        .map(|g| format!(", below: {g}"))
        .unwrap_or_default();

    // Right column: room \ time \ cost (Typst line-break inside cell)
    let right_items = build_right_column(&room_str, &time_range, panel.cost.as_deref());

    // Credits on their own line below the panel name
    let credits_line = if !panel.credits.is_empty() {
        format!(
            "\\\n#text(size: {SECONDARY_SIZE}, style: \"italic\")[{}]",
            escape_typst(&panel.credits.join(", "))
        )
    } else {
        String::new()
    };

    // Header grid: 1fr left (name + credits), auto right (room/time/cost stacked)
    let mut block = format!(
        "#block(breakable: false{style_attrs}{gap_attr})[\n\
         #grid(columns: (1fr, auto), align: (top + left, top + right),\n\
           [*{name}*{credits}],\n\
           [#text(size: {secondary_size})[{right}]],\n\
         )\n",
        style_attrs = style_attrs,
        gap_attr = gap_attr,
        name = escape_typst(&panel.name),
        credits = credits_line,
        right = right_items,
        secondary_size = SECONDARY_SIZE,
    );

    // Description - uses base font size (inherited from preamble)
    let desc_text = panel
        .description
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("Description pending");
    block.push_str(&format!("\n{}\n", escape_typst(desc_text)));

    // Notes / workshop notice block
    let notice = workshop_cap_notice(data, panel);
    let has_notice = notice.is_some()
        || panel.note.as_deref().is_some_and(|n| !n.is_empty())
        || panel.is_full
        || panel.difficulty.as_deref().is_some_and(|d| !d.is_empty());

    if has_notice {
        let mut note_parts: Vec<String> = vec![];
        if let Some(n) = notice {
            note_parts.push(n);
        }
        if let Some(note) = panel.note.as_deref().filter(|n| !n.is_empty()) {
            note_parts.push(format!("#text(style: \"italic\")[{}]", escape_typst(note)));
        }
        if panel.is_full {
            note_parts.push(escape_typst("This workshop is full."));
        }
        if let Some(diff) = panel.difficulty.as_deref().filter(|d| !d.is_empty()) {
            note_parts.push(escape_typst(&format!("Difficulty level: {}", diff)));
        }
        // Notes use base font size
        block.push_str(&format!("\n{}\n", note_parts.join(" ")));
    }

    // Prereq block - uses base font size
    if let Some(prereq) = panel.prereq.as_deref().filter(|p| !p.is_empty()) {
        let prereq_content = resolve_prereq(prereq, day_date, &data.panels);
        block.push_str(&format!("\n{}\n", prereq_content));
    }

    // Cross-references (parts and reruns) - use base font size
    let xrefs = build_cross_refs(panel, by_base);
    for xref in &xrefs {
        block.push_str(&format!("\n{}\n", escape_typst(xref)));
    }

    if let Some(lbl) = typst_label {
        block.push_str(&format!("] <{}>\n\n", lbl));
    } else {
        block.push_str("]\n\n");
    }
    block
}

/// Build the stacked right-column content for the panel header grid.
///
/// Items are joined with Typst's `\ ` line-break so they stack vertically.
fn build_right_column(room: &str, time_range: &str, cost: Option<&str>) -> String {
    let mut parts: Vec<String> = vec![];
    if !room.is_empty() {
        parts.push(escape_typst(room));
    }
    if !time_range.is_empty() {
        parts.push(escape_typst(time_range));
    }
    if let Some(c) = cost.filter(|c| !c.is_empty()) {
        parts.push(format!("*{}*", escape_typst(c)));
    }
    parts.join(" \\ \n")
}

/// Generate the bold workshop/premium/capacity notice string, or `None`.
fn workshop_cap_notice(data: &ScheduleData, panel: &Panel) -> Option<String> {
    let cap_suffix = panel
        .capacity
        .as_deref()
        .filter(|c| !c.is_empty())
        .map(|c| format!(" (Capacity: {})", c))
        .unwrap_or_default();

    let is_workshop = panel
        .panel_type
        .as_ref()
        .and_then(|pt| data.panel_types.get(pt.as_str()))
        .is_some_and(|pt| pt.is_workshop);

    if panel.is_premium {
        Some(format!(
            "*Premium workshop:*{} Requires a separate purchase.",
            cap_suffix
        ))
    } else if is_workshop {
        Some(format!("*Workshop:*{}", cap_suffix))
    } else if panel.capacity.as_deref().is_some_and(|c| !c.is_empty()) {
        Some(format!("*Limited space:*{}", cap_suffix))
    } else {
        None
    }
}

/// Resolve the `prereq` field into a Typst-safe string.
///
/// Tokens that parse as a valid `PanelUniqId` and match a panel are shown as
/// `"Prereq: Panel Name: Saturday 4:00 PM"`.  Unresolved tokens are shown as
/// italic text.
fn resolve_prereq(prereq: &str, day_date: &str, all_panels: &[Panel]) -> String {
    let tokens: Vec<&str> = prereq
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let mut resolved: Vec<String> = vec![];
    let mut unresolved: Vec<&str> = vec![];

    for token in &tokens {
        if let Some(uid) = PanelUniqId::parse(token) {
            let base = uid.base_id();
            let full = uid.full_id();
            let found = all_panels
                .iter()
                .find(|p| p.id == full)
                .or_else(|| all_panels.iter().find(|p| p.base_id == base));
            if let Some(p) = found {
                let time_label = p
                    .start_time
                    .as_deref()
                    .map(|t| time_fmt::format_weekday_time(t, day_date))
                    .unwrap_or_default();
                resolved.push(escape_typst(&format!("Prereq: {}: {}", p.name, time_label)));
            } else {
                unresolved.push(token);
            }
        } else {
            unresolved.push(token);
        }
    }

    let mut parts: Vec<String> = resolved;
    if !unresolved.is_empty() {
        parts.push(format!(
            "#text(style: \"italic\")[{}]",
            escape_typst(&unresolved.join("; "))
        ));
    }
    parts.join(" ")
}

/// Build cross-reference lines for a panel (parts or rerun sessions).
fn build_cross_refs<'a>(
    panel: &'a Panel,
    by_base: &HashMap<&'a str, Vec<&'a Panel>>,
) -> Vec<String> {
    let related: &[&Panel] = by_base
        .get(panel.base_id.as_str())
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let others: Vec<&Panel> = related
        .iter()
        .copied()
        .filter(|p| p.id != panel.id)
        .collect();

    if others.is_empty() {
        return vec![];
    }

    let mut refs: Vec<String> = vec![];

    if panel.part_num.is_some() {
        let mut by_part: HashMap<i32, Vec<&Panel>> = HashMap::new();
        for p in &others {
            by_part.entry(p.part_num.unwrap_or(1)).or_default().push(p);
        }
        let mut part_keys: Vec<i32> = by_part.keys().copied().collect();
        part_keys.sort_unstable();
        for part in part_keys {
            let mut sessions = by_part[&part].clone();
            sessions.sort_by_key(|p| p.start_time.as_deref().unwrap_or(""));
            let mut first = true;
            for p in sessions {
                let label = if first {
                    format!("Part {}", part)
                } else {
                    format!("or Part {}", part)
                };
                first = false;
                let time_str = p
                    .start_time
                    .as_deref()
                    .map(|t| time_fmt::format_weekday_time(t, ""))
                    .unwrap_or_default();
                refs.push(format!("{}: {}", label, time_str));
            }
        }
    } else if panel.session_num.is_some() {
        let mut sorted = others.clone();
        sorted.sort_by_key(|p| p.start_time.as_deref().unwrap_or(""));
        for p in sorted {
            let time_str = p
                .start_time
                .as_deref()
                .map(|t| time_fmt::format_weekday_time(t, ""))
                .unwrap_or_default();
            refs.push(format!("Rerun at: {}", time_str));
        }
    }

    refs
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /*
     * Copyright (c) 2026 Daphne Pfister
     * SPDX-License-Identifier: BSD-2-Clause
     * See LICENSE file for full license text
     */

    use super::*;
    use crate::model::{Meta, ScheduleData};

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
            panel_types: std::collections::HashMap::new(),
            timeline: vec![],
            presenters: vec![],
        }
    }

    #[test]
    fn test_workshop_notice_premium() {
        let data = empty_schedule();
        let panel = Panel {
            id: "WS001P1".into(),
            base_id: "WS001".into(),
            name: "Test Workshop".into(),
            is_premium: true,
            capacity: Some("12".into()),
            ..Panel::default()
        };
        let notice = workshop_cap_notice(&data, &panel);
        assert!(notice.is_some());
        let n = notice.unwrap();
        assert!(n.contains("Premium workshop:"));
        assert!(n.contains("Capacity: 12"));
        assert!(n.contains("Requires a separate purchase."));
    }

    #[test]
    fn test_workshop_notice_none_for_free_panel() {
        let data = empty_schedule();
        let panel = Panel {
            id: "GP001".into(),
            base_id: "GP001".into(),
            name: "Free Panel".into(),
            ..Panel::default()
        };
        assert!(workshop_cap_notice(&data, &panel).is_none());
    }
}
