/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Layout job configuration: TOML schema, preset resolution, and conversion to
//! typed [`schedule_layout`] config values.
//!
//! This module is the single source of truth for what the layout TOML keys mean:
//! - [`LayoutDefaults`] / [`JobConfig`] hold the raw (string) TOML values.
//! - [`JobConfig::to_layout_config`] converts a fully-resolved job into the typed
//!   [`schedule_layout::config::LayoutConfig`] used by the renderer.
//! - [`apply_layout_arg`] maps `--layout.<key>=<value>` CLI flags onto a
//!   `JobConfig` in progress.
//!
//! # Presets and Imports
//!
//! Named presets can be defined using `[presets.name]` sections:
//!
//! ```toml
//! [presets.workshop_base]
//! content = "description_only"
//! section_split = "none"
//! panel_filter = "workshops"
//! cards = true
//!
//! [presets.large_print]
//! paper = "tabloid"
//! base_font_pt = "14pt"
//!
//! [[jobs]]
//! import = ["workshop_base", "large_print"]
//! stem = "workshops-large"
//! orientation = "portrait"
//! ```
//!
//! Jobs and presets can import one or more presets using the `import` field.
//! Later imports override earlier ones, and job-specific settings override
//! all imported values.
//!
//! # Private (staff) jobs
//!
//! A job may set `include_private = true` to render the private view: private
//! panels and unlisted (uncredited) presenters are included, so per-presenter
//! sections produce postcards for unlisted guests. The default (`false`) is the
//! public view. cosam-convert builds whichever views the configured jobs need.
//!
//! ```toml
//! [[jobs]]
//! stem = "staff-postcards"
//! content = "panel_list"
//! section_split = "presenter"
//! include_private = true
//! ```
//!
//! # Custom Brand Configuration
//!
//! Individual jobs can specify a different brand.toml file:
//!
//! ```toml
//! [[jobs]]
//! stem = "special-edition"
//! brand_config = "config/brand-special.toml"
//! paper = "tabloid"
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "layout")]
use schedule_layout::{
    color::ColorMode,
    config::{
        ContentMode, FooterMode, LayoutConfig, LayoutFormat, Orientation, PanelFilter, PaperSize,
        SectionSplit, TimeSplit,
    },
};

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum LayoutConfigError {
    #[error("I/O error reading layout config: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Preset cycle detected: {0}")]
    PresetCycle(String),
    #[error("Unknown preset: {0}")]
    UnknownPreset(String),
}

// ── TOML schema ───────────────────────────────────────────────────────────────

/// Layout configuration loaded from `config/layout.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutDefaults {
    /// Layout jobs to generate. If empty, uses hardcoded defaults.
    pub jobs: Vec<JobConfig>,
    /// Named presets that can be imported by jobs or other presets.
    pub presets: HashMap<String, JobConfig>,
}

/// Layout job configuration for generating a specific output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct JobConfig {
    /// Paper size: "letter", "legal", "tabloid", "super_b", "poster", "postcard"
    pub paper: String,
    /// Output format: "typst" (default, PDF via Typst) or "idml" (Adobe InDesign).
    /// IDML requires building with the `idml` feature. Optional.
    pub format: Option<String>,
    /// Content: "both" (default), "grid_only", "description_only", "panel_list"
    pub content: Option<String>,
    /// Entity (section) split: "none" (default), "room", "presenter".
    pub section_split: Option<String>,
    /// Time split: "none" (default), "day", "half_day", "timeline".
    /// - "none": no time split (one section, or one per entity if section_split is set)
    /// - "day": one section per calendar day
    /// - "half_day": one section per AM/PM half (geometric noon boundary)
    /// - "timeline": one section per timeline/SPLIT entry in the schedule data
    ///
    /// Grid-bearing modes (`both`, `grid_only`) require an explicit time split.
    pub time_split: Option<String>,
    /// Deprecated combined split key kept for backward compatibility.
    /// Expands to `section_split` + `time_split`; prefer the two new keys.
    /// Accepted values: "none", "day", "half_day", "room", "room_day",
    ///                  "presenter", "presenter_day".
    #[serde(alias = "split_by", skip_serializing)]
    pub split: Option<String>,
    /// Orientation: "portrait" or "landscape"
    pub orientation: String,
    /// File stem prefix (e.g., "schedule", "desc", "workshops")
    pub stem: String,
    /// Panel filter: "all" (default), "workshops", "premium". Optional.
    pub panel_filter: Option<String>,
    /// Include private panels and unlisted (uncredited) presenters in this job.
    /// Defaults to `false` (public view). Optional.
    pub include_private: Option<bool>,
    /// Color mode: "color" (default) or "bw". Optional.
    pub color_mode: Option<String>,
    /// Page footer: "full" (default), "timestamp_only", "none". Optional.
    pub footer: Option<String>,
    /// Insert blank pages so each section starts on an odd page. Optional.
    pub double_sided: Option<bool>,
    /// Header text (left for 1-D splits, right for "none"). Optional.
    pub header_text: Option<String>,
    /// Column-count override. If unset, the content/paper default is used.
    pub columns: Option<u32>,
    /// Optional font size override (e.g., "13.2pt"). Uses defaults if not specified.
    pub base_font_pt: Option<String>,
    /// Optional grid event text size override (e.g., "8pt"). If not set, uses base_font_pt.
    pub grid_font_pt: Option<String>,
    /// Page background color: hex (`"#f2f2f2"`), `"luma(95%)"`, or a named color.
    pub page_fill: Option<String>,
    /// Fill for empty grid cells (keeps them from blending into a tinted page).
    pub empty_grid_fill: Option<String>,
    /// Render description panels as bordered cards instead of the left-bar style.
    pub cards: Option<bool>,
    /// Card background color when `cards` is set (defaults to white).
    pub card_fill: Option<String>,
    /// Override the gutter between body-text columns (e.g. `"0.25in"`).
    pub column_gap: Option<String>,
    /// Gap between cards (with `cards`); `"column"` means "match the column gutter".
    pub card_gap: Option<String>,
    /// Presets to import and merge into this job. Later imports override earlier ones.
    pub import: Vec<String>,
    /// Custom brand.toml path for this job. If not set, uses the global brand config.
    pub brand_config: Option<String>,
    /// Logo to show in the page header.
    /// - `"brand"` (default when unset) — resolves the `"brand"` alias from `[logos]`.
    /// - `"none"` — suppress the logo entirely.
    /// - Any other string — looked up as a named alias in `[logos]`, then as a bare
    ///   filename within `logo_dir`.
    pub logo: Option<String>,
    /// Override the banner text size (e.g. `"18pt"`). Defaults to 28 pt when unset.
    /// Useful for postcards or jobs with long presenter names.
    pub banner_text_pt: Option<String>,
}

// ── Preset resolution ─────────────────────────────────────────────────────────

impl JobConfig {
    /// Merge `other` into `self`; `other`'s values win only when explicitly set.
    fn merge_from(&mut self, other: &JobConfig) {
        if !other.paper.is_empty() {
            self.paper = other.paper.clone();
        }
        if other.format.is_some() {
            self.format = other.format.clone();
        }
        if other.content.is_some() {
            self.content = other.content.clone();
        }
        if other.section_split.is_some() {
            self.section_split = other.section_split.clone();
        }
        if other.time_split.is_some() {
            self.time_split = other.time_split.clone();
        }
        if other.split.is_some() {
            self.split = other.split.clone();
        }
        if !other.orientation.is_empty() {
            self.orientation = other.orientation.clone();
        }
        if !other.stem.is_empty() {
            self.stem = other.stem.clone();
        }
        if other.panel_filter.is_some() {
            self.panel_filter = other.panel_filter.clone();
        }
        if other.include_private.is_some() {
            self.include_private = other.include_private;
        }
        if other.color_mode.is_some() {
            self.color_mode = other.color_mode.clone();
        }
        if other.footer.is_some() {
            self.footer = other.footer.clone();
        }
        if other.double_sided.is_some() {
            self.double_sided = other.double_sided;
        }
        if other.header_text.is_some() {
            self.header_text = other.header_text.clone();
        }
        if other.columns.is_some() {
            self.columns = other.columns;
        }
        if other.base_font_pt.is_some() {
            self.base_font_pt = other.base_font_pt.clone();
        }
        if other.grid_font_pt.is_some() {
            self.grid_font_pt = other.grid_font_pt.clone();
        }
        if other.page_fill.is_some() {
            self.page_fill = other.page_fill.clone();
        }
        if other.empty_grid_fill.is_some() {
            self.empty_grid_fill = other.empty_grid_fill.clone();
        }
        if other.cards.is_some() {
            self.cards = other.cards;
        }
        if other.card_fill.is_some() {
            self.card_fill = other.card_fill.clone();
        }
        if other.column_gap.is_some() {
            self.column_gap = other.column_gap.clone();
        }
        if other.card_gap.is_some() {
            self.card_gap = other.card_gap.clone();
        }
        if other.brand_config.is_some() {
            self.brand_config = other.brand_config.clone();
        }
        if other.logo.is_some() {
            self.logo = other.logo.clone();
        }
        if other.banner_text_pt.is_some() {
            self.banner_text_pt = other.banner_text_pt.clone();
        }
    }

    /// Resolve this job by applying all imported presets in order.
    pub fn resolve(
        &self,
        presets: &HashMap<String, JobConfig>,
        resolution_stack: &mut Vec<String>,
    ) -> Result<JobConfig, LayoutConfigError> {
        let mut resolved = JobConfig::default();

        for preset_name in &self.import {
            if resolution_stack.contains(preset_name) {
                return Err(LayoutConfigError::PresetCycle(format!(
                    "Cycle detected: {} -> {}",
                    resolution_stack.join(" -> "),
                    preset_name
                )));
            }
            let preset = presets
                .get(preset_name)
                .ok_or_else(|| LayoutConfigError::UnknownPreset(preset_name.clone()))?;
            resolution_stack.push(preset_name.clone());
            let resolved_preset = preset.resolve(presets, resolution_stack)?;
            resolution_stack.pop();
            resolved.merge_from(&resolved_preset);
        }

        // Job's own settings win over anything from presets.
        resolved.merge_from(self);
        Ok(resolved)
    }
}

// ── String → typed-value parsers ──────────────────────────────────────────────
//
// All parsers are `pub(crate)` so `main.rs` can call them directly for the
// command-line `--layout.*` path if ever needed, but they are not part of the
// public API.

#[cfg(feature = "layout")]
pub(crate) fn parse_paper(s: &str) -> PaperSize {
    match s {
        "letter" => PaperSize::Letter,
        "legal" => PaperSize::Legal,
        "tabloid" => PaperSize::Tabloid,
        "super_b" | "superb" => PaperSize::SuperB,
        "poster" => PaperSize::Poster,
        "postcard" => PaperSize::Postcard4x6,
        "" => PaperSize::Tabloid,
        other => {
            eprintln!(
                "warning: unknown paper '{other}'; expected one of: letter, legal, tabloid, \
                 super_b, poster, postcard — using 'tabloid'"
            );
            PaperSize::Tabloid
        }
    }
}

#[cfg(feature = "layout")]
pub(crate) fn parse_orientation(s: &str) -> Orientation {
    match s {
        "portrait" => Orientation::Portrait,
        "landscape" | "" => Orientation::Landscape,
        other => {
            eprintln!(
                "warning: unknown orientation '{other}'; expected one of: portrait, landscape \
                 — using 'landscape'"
            );
            Orientation::Landscape
        }
    }
}

#[cfg(feature = "layout")]
pub(crate) fn parse_color_mode(s: Option<&str>) -> ColorMode {
    match s {
        Some("bw") | Some("grayscale") => ColorMode::Bw,
        Some("color") | None => ColorMode::Color,
        Some(other) => {
            eprintln!(
                "warning: unknown color_mode '{other}'; expected one of: color, bw — using 'color'"
            );
            ColorMode::Color
        }
    }
}

#[cfg(feature = "layout")]
pub(crate) fn parse_panel_filter(s: Option<&str>) -> PanelFilter {
    match s {
        Some("workshops") => PanelFilter::Workshops,
        Some("premium") => PanelFilter::Premium,
        None | Some("all") => PanelFilter::All,
        Some(other) => {
            eprintln!(
                "warning: unknown panel_filter '{other}'; expected one of: all, workshops, \
                 premium — using 'all'"
            );
            PanelFilter::All
        }
    }
}

#[cfg(feature = "layout")]
pub(crate) fn parse_footer(s: Option<&str>) -> FooterMode {
    match s {
        Some("timestamp_only") | Some("timestamp-only") => FooterMode::TimestampOnly,
        Some("none") => FooterMode::None,
        None | Some("full") => FooterMode::Full,
        Some(other) => {
            eprintln!(
                "warning: unknown footer '{other}'; expected one of: full, timestamp_only, \
                 none — using 'full'"
            );
            FooterMode::Full
        }
    }
}

#[cfg(feature = "layout")]
pub(crate) fn parse_format(s: Option<&str>) -> LayoutFormat {
    match s.map(str::trim) {
        Some("idml") => LayoutFormat::Idml,
        _ => LayoutFormat::Typst,
    }
}

/// Parse `section_split` key: "none", "room", "presenter".
#[cfg(feature = "layout")]
fn parse_section_split(s: Option<&str>) -> Option<SectionSplit> {
    match s {
        Some("none") | None => None,
        Some("room") => Some(SectionSplit::Room),
        Some("presenter") => Some(SectionSplit::Presenter),
        Some(other) => {
            eprintln!(
                "warning: unknown section_split '{other}'; expected one of: none, room, \
                 presenter — ignoring"
            );
            None
        }
    }
}

/// Parse `time_split` key: "none", "day", "half_day", "timeline".
#[cfg(feature = "layout")]
fn parse_time_split(s: Option<&str>) -> Option<TimeSplit> {
    match s {
        Some("none") | None => None,
        Some("day") => Some(TimeSplit::Day),
        Some("half_day") | Some("half-day") => Some(TimeSplit::HalfDay),
        // "timeline" splits on the schedule's actual timeline/SPLIT panel entries.
        Some("timeline") => Some(TimeSplit::Timeline),
        Some(other) => {
            eprintln!(
                "warning: unknown time_split '{other}'; expected one of: none, day, half_day, \
                 timeline — ignoring"
            );
            None
        }
    }
}

/// Expand the deprecated combined `split` key into (section_split, time_split) strings.
/// Emits a deprecation warning. Explicit `section_split`/`time_split` fields
/// take priority over the expanded values when the caller merges them.
#[cfg(feature = "layout")]
fn expand_deprecated_split(split: &str) -> (Option<&'static str>, Option<&'static str>) {
    match split {
        "none" => (None, None),
        "day" => (None, Some("day")),
        "half_day" | "half-day" => (None, Some("half_day")),
        "room" => (Some("room"), None),
        "room_day" | "room-day" => (Some("room"), Some("day")),
        "presenter" => (Some("presenter"), None),
        "presenter_day" | "presenter-day" => (Some("presenter"), Some("day")),
        other => {
            eprintln!(
                "warning: unknown split '{other}'; expected one of: none, day, half_day, \
                 room, room_day, presenter, presenter_day — ignoring"
            );
            (None, None)
        }
    }
}

/// Resolve a job's section and time split, honouring the deprecated `split` key
/// as a fallback when the explicit independent keys are absent.
#[cfg(feature = "layout")]
fn resolve_splits(job: &JobConfig) -> (Option<SectionSplit>, Option<TimeSplit>) {
    let (dep_section, dep_time) = match &job.split {
        Some(s) => {
            eprintln!(
                "warning: job '{}': 'split' is deprecated; use 'section_split' and \
                 'time_split' instead",
                job.stem
            );
            expand_deprecated_split(s)
        }
        None => (None, None),
    };
    let section_str = job.section_split.as_deref().or(dep_section);
    let time_str = job.time_split.as_deref().or(dep_time);
    (parse_section_split(section_str), parse_time_split(time_str))
}

/// Build a `ContentMode` from the resolved splits and the `content` key string.
/// Grid-bearing modes require a time split; falls back to `Day` with an error
/// message if none is set.
#[cfg(feature = "layout")]
fn build_content_mode(
    content: Option<&str>,
    section: Option<SectionSplit>,
    time: Option<TimeSplit>,
    stem: &str,
) -> ContentMode {
    let needs_time = matches!(
        content,
        Some("grid_only") | Some("grid-only") | Some("both") | None
    );
    if needs_time && time.is_none() {
        eprintln!(
            "error: job '{stem}': content mode '{}' requires a time split \
             (day, half_day, or timeline); falling back to 'day'",
            content.unwrap_or("both")
        );
    }
    let time_req = time.unwrap_or(TimeSplit::Day);
    match content {
        Some("grid_only") | Some("grid-only") => ContentMode::GridOnly {
            section,
            time: time_req,
        },
        Some("description_only") | Some("description-only") => {
            ContentMode::DescriptionOnly { section, time }
        }
        Some("panel_list") | Some("panel-list") => ContentMode::PanelList { section, time },
        None | Some("both") => ContentMode::Both {
            section,
            time: time_req,
        },
        Some(other) => {
            eprintln!(
                "warning: job '{stem}': unknown content '{other}'; expected one of: both, \
                 grid_only, description_only, panel_list — using 'both'"
            );
            ContentMode::Both {
                section,
                time: time_req,
            }
        }
    }
}

// ── JobConfig → LayoutConfig conversion ──────────────────────────────────────

#[cfg(feature = "layout")]
impl JobConfig {
    /// Convert a fully-resolved `JobConfig` into a typed [`LayoutConfig`].
    ///
    /// Returns `(LayoutConfig, stem)`.  The stem is the base filename (no
    /// extension) for the output file; callers append qualifiers as needed.
    pub fn to_layout_config(&self) -> (LayoutConfig, String) {
        let (section, time) = resolve_splits(self);
        let content = build_content_mode(self.content.as_deref(), section, time, &self.stem);
        let config = LayoutConfig {
            paper: parse_paper(&self.paper),
            format: parse_format(self.format.as_deref()),
            content,
            panel_filter: parse_panel_filter(self.panel_filter.as_deref()),
            include_private: self.include_private.unwrap_or(false),
            orientation: parse_orientation(&self.orientation),
            color_mode: parse_color_mode(self.color_mode.as_deref()),
            columns: self.columns,
            footer: parse_footer(self.footer.as_deref()),
            double_sided: self.double_sided.unwrap_or(false),
            header_text: self.header_text.clone(),
            base_font_pt: self.base_font_pt.clone(),
            grid_font_pt: self.grid_font_pt.clone(),
            page_fill: self.page_fill.clone(),
            empty_grid_fill: self.empty_grid_fill.clone(),
            cards: self.cards.unwrap_or(false),
            card_fill: self.card_fill.clone(),
            column_gap: self.column_gap.clone(),
            card_gap: self.card_gap.clone(),
            // Default to "brand" so jobs without an explicit `logo` key still
            // show the brand logo.
            logo: Some(self.logo.clone().unwrap_or_else(|| "brand".to_string())),
            banner_text_pt: self.banner_text_pt.clone(),
        };
        (config, self.stem.clone())
    }
}

// ── CLI argument application ──────────────────────────────────────────────────

/// Parse a `--layout.<bool-key>` flag value.  A bare flag (no `=value`) is
/// treated as `true`.
#[cfg(feature = "layout")]
pub fn parse_layout_bool(key: &str, value: Option<&str>) -> Result<bool> {
    match value {
        None => Ok(true),
        Some(v) => match v.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Ok(true),
            "false" | "no" | "0" | "off" => Ok(false),
            other => anyhow::bail!("--layout.{key} expects a boolean, got '{other}'"),
        },
    }
}

/// Apply one `--layout.<key>[=<value>]` CLI flag to an in-progress `JobConfig`.
///
/// `key` is the part after `--layout.`, `value` is the part after the first `=`
/// (absent for bare boolean flags).  Keys mirror the TOML field names; hyphens
/// are accepted as a synonym for underscores.
#[cfg(feature = "layout")]
pub fn apply_layout_arg(job: &mut JobConfig, key: &str, value: Option<&str>) -> Result<()> {
    let str_val = || -> Result<String> {
        value
            .map(|v| v.to_string())
            .ok_or_else(|| anyhow::anyhow!("--layout.{key} requires a value (--layout.{key}=...)"))
    };

    let normalized = key.replace('-', "_");
    match normalized.as_str() {
        "paper" => job.paper = str_val()?,
        "format" => job.format = Some(str_val()?),
        "content" => job.content = Some(str_val()?),
        "section_split" | "section-split" => job.section_split = Some(str_val()?),
        "time_split" | "time-split" => job.time_split = Some(str_val()?),
        // Deprecated combined split key: kept for backward compatibility.
        "split" | "split_by" => job.split = Some(str_val()?),
        "orientation" => job.orientation = str_val()?,
        "stem" => job.stem = str_val()?,
        "panel_filter" => job.panel_filter = Some(str_val()?),
        "include_private" => job.include_private = Some(parse_layout_bool(key, value)?),
        "color_mode" => job.color_mode = Some(str_val()?),
        "footer" => job.footer = Some(str_val()?),
        "double_sided" => job.double_sided = Some(parse_layout_bool(key, value)?),
        "header_text" => job.header_text = Some(str_val()?),
        "columns" => {
            job.columns = Some(
                str_val()?
                    .parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("--layout.columns must be a positive integer"))?,
            )
        }
        "base_font_pt" => job.base_font_pt = Some(str_val()?),
        "grid_font_pt" => job.grid_font_pt = Some(str_val()?),
        "page_fill" => job.page_fill = Some(str_val()?),
        "empty_grid_fill" => job.empty_grid_fill = Some(str_val()?),
        "cards" => job.cards = Some(parse_layout_bool(key, value)?),
        "card_fill" => job.card_fill = Some(str_val()?),
        "column_gap" => job.column_gap = Some(str_val()?),
        "card_gap" => job.card_gap = Some(str_val()?),
        "import" => job.import.push(str_val()?),
        "brand_config" => job.brand_config = Some(str_val()?),
        "logo" => job.logo = Some(str_val()?),
        "banner_text_pt" => job.banner_text_pt = Some(str_val()?),
        other => anyhow::bail!("unknown --layout.{other} key"),
    }
    Ok(())
}

// ── LayoutDefaults loading ────────────────────────────────────────────────────

impl LayoutDefaults {
    /// Load from a TOML file. Returns empty defaults if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, LayoutConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        Self::from_str(&text)
    }

    /// Parse from a TOML string.
    pub fn from_str(text: &str) -> Result<Self, LayoutConfigError> {
        let defaults: LayoutDefaults = toml::from_str(text)?;
        Ok(defaults)
    }

    /// Return the built-in default layout configuration.
    /// Parsed from `config/layout-default.toml` at compile time and cached.
    pub fn default_layout() -> LayoutDefaults {
        static DEFAULT_LAYOUT: LazyLock<LayoutDefaults> = LazyLock::new(|| {
            let text = include_str!("../../../config/layout-default.toml");
            LayoutDefaults::from_str(text)
                .expect("embedded layout-default.toml should be valid TOML")
        });
        DEFAULT_LAYOUT.clone()
    }

    /// Resolve all jobs, applying preset imports.
    /// Returns `Vec<(resolved_job, maybe_brand_config_path)>`.
    pub fn resolve_jobs(&self) -> Result<Vec<(JobConfig, Option<String>)>, LayoutConfigError> {
        self.jobs
            .iter()
            .map(|job| {
                let resolved = job.resolve(&self.presets, &mut Vec::new())?;
                let brand = resolved.brand_config.clone();
                Ok((resolved, brand))
            })
            .collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_empty() {
        let defaults = LayoutDefaults::default();
        assert!(defaults.jobs.is_empty());
    }

    #[test]
    fn test_default_layout_has_jobs() {
        let defaults = LayoutDefaults::default_layout();
        assert!(!defaults.jobs.is_empty());
    }

    #[test]
    fn test_parse_toml_jobs() {
        let toml = r#"
[[jobs]]
content = "grid_only"
paper = "tabloid"
time_split = "half_day"
orientation = "landscape"
stem = "schedule"
panel_filter = "workshops"
color_mode = "bw"
double_sided = true
columns = 5
base_font_pt = "10pt"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        assert_eq!(defaults.jobs.len(), 1);
        assert_eq!(defaults.jobs[0].content, Some("grid_only".to_string()));
        assert_eq!(defaults.jobs[0].time_split, Some("half_day".to_string()));
        assert_eq!(defaults.jobs[0].panel_filter, Some("workshops".to_string()));
        assert_eq!(defaults.jobs[0].color_mode, Some("bw".to_string()));
        assert_eq!(defaults.jobs[0].double_sided, Some(true));
        assert_eq!(defaults.jobs[0].columns, Some(5));
        assert_eq!(defaults.jobs[0].base_font_pt, Some("10pt".to_string()));
    }

    #[test]
    fn test_parse_toml_deprecated_split_alias() {
        // The old combined `split_by` / `split` keys must still be accepted.
        let toml = r#"
[[jobs]]
content = "grid_only"
paper = "tabloid"
split_by = "half_day"
orientation = "landscape"
stem = "schedule"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        // Deprecated key is parsed into `split` field.
        assert_eq!(defaults.jobs[0].split, Some("half_day".to_string()));
        // The explicit independent keys are absent.
        assert_eq!(defaults.jobs[0].time_split, None);
    }

    #[test]
    fn test_parse_toml_independent_splits() {
        let toml = r#"
[[jobs]]
content = "both"
paper = "tabloid"
section_split = "room"
time_split = "day"
orientation = "landscape"
stem = "room-signs"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        assert_eq!(defaults.jobs[0].section_split, Some("room".to_string()));
        assert_eq!(defaults.jobs[0].time_split, Some("day".to_string()));
        assert_eq!(defaults.jobs[0].split, None);
    }

    #[test]
    fn test_preset_parsing() {
        let toml = r#"
[presets.workshop_base]
content = "description_only"
panel_filter = "workshops"
cards = true
paper = "letter"
orientation = "portrait"
stem = "workshop"

[[jobs]]
import = ["workshop_base"]
stem = "my-workshops"
paper = "tabloid"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        assert_eq!(defaults.presets.len(), 1);
        assert!(defaults.presets.contains_key("workshop_base"));
        assert_eq!(defaults.jobs[0].import, vec!["workshop_base"]);
    }

    #[test]
    fn test_preset_resolution() {
        let toml = r#"
[presets.base]
paper = "letter"
orientation = "portrait"
stem = "base"
time_split = "day"
content = "both"

[presets.large]
paper = "tabloid"
base_font_pt = "14pt"

[[jobs]]
import = ["base", "large"]
stem = "final"
orientation = "landscape"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        let resolved = defaults.resolve_jobs().unwrap();

        assert_eq!(resolved.len(), 1);
        let (job, _) = &resolved[0];
        // From base: time_split, content
        assert_eq!(job.time_split, Some("day".to_string()));
        assert_eq!(job.content, Some("both".to_string()));
        // From large (overrides base): paper, base_font_pt
        assert_eq!(job.paper, "tabloid");
        assert_eq!(job.base_font_pt, Some("14pt".to_string()));
        // From job itself (overrides all): stem, orientation
        assert_eq!(job.stem, "final");
        assert_eq!(job.orientation, "landscape");
    }

    #[test]
    fn test_preset_cycle_detection() {
        let toml = r#"
[presets.a]
import = ["b"]
paper = "letter"

[presets.b]
import = ["a"]
paper = "tabloid"

[[jobs]]
import = ["a"]
stem = "test"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        let result = defaults.resolve_jobs();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle detected"));
    }

    #[test]
    fn test_unknown_preset() {
        let toml = r#"
[[jobs]]
import = ["nonexistent"]
stem = "test"
"#;
        let defaults = LayoutDefaults::from_str(toml).unwrap();
        let result = defaults.resolve_jobs();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }
}
